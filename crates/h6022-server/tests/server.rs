//! Test di integrazione del server: `/health` con HTTP scritto a mano, poi il client MCP.

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::time::Duration;

use h6022_core::sim::SimulatedSource;
use h6022_core::{Engine, EngineConfig, Slope, TriggerConfig, TriggerKind, TriggerMode};
use h6022_server::{ServerConfig, spawn};
use h6022_usb::{AcquisitionConfig, Channel, ChannelMode, Coupling, SampleRate, VoltageRange};
use rmcp::ServiceExt;
use rmcp::transport::StreamableHttpClientTransport;

fn config() -> EngineConfig {
    EngineConfig {
        acquisition: AcquisitionConfig {
            sample_rate: SampleRate::nearest(1e6),
            mode: ChannelMode::Dual,
            range: [VoltageRange::V5; 2],
            coupling: [Coupling::Dc; 2],
        },
        trigger: TriggerConfig {
            source: Channel::Ch1,
            kind: TriggerKind::Edge { slope: Slope::Rising },
            level: 1.5,
            hysteresis: 0.1,
            holdoff: 0.0,
        },
        mode: TriggerMode::Auto,
        record_length: 1000,
        pretrigger: 0.25,
        probe: [1.0, 1.0],
    }
}

fn get_health(addr: SocketAddr) -> String {
    let mut stream = TcpStream::connect(addr).expect("connessione a /health");
    stream.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
    write!(stream, "GET /health HTTP/1.1\r\nHost: {addr}\r\nConnection: close\r\n\r\n").unwrap();
    let mut response = String::new();
    stream.read_to_string(&mut response).unwrap();
    response
}

#[test]
fn health_mcp_e_shutdown() {
    let engine = Engine::start(Box::new(SimulatedSource::bench_default()), config()).unwrap();
    let addr = SocketAddr::from(([127, 0, 0, 1], 0));
    let server = spawn(engine.clone(), "simulator", ServerConfig { addr }).unwrap();
    let bound = server.addr();
    assert_ne!(bound.port(), 0);

    let health = get_health(bound);
    assert!(health.starts_with("HTTP/1.1 200"), "risposta: {health}");
    assert!(health.contains("ok"), "risposta: {health}");

    let runtime = tokio::runtime::Runtime::new().unwrap();
    runtime.block_on(async {
        let transport = StreamableHttpClientTransport::from_uri(format!("http://{bound}/mcp"));
        let client = ().serve(transport).await.expect("handshake MCP");

        let tools = client.list_all_tools().await.expect("list_tools");
        let tools_json = serde_json::to_string(&tools).unwrap();
        assert!(tools_json.contains("\"device_status\""), "tool: {tools_json}");
        assert!(tools_json.contains("\"capture\""), "tool: {tools_json}");

        let status = client
            .call_tool(rmcp::model::CallToolRequestParams::new("device_status"))
            .await
            .expect("call_tool device_status");
        let status_json = serde_json::to_string(&status).unwrap();
        assert!(status_json.contains("simulator"), "risposta: {status_json}");

        client.cancel().await.ok();
    });

    server.shutdown();
    let _ = engine.shutdown();
}
