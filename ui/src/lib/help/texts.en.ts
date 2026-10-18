// Spiegazioni dei controlli in inglese. Stesse chiavi e stesso limite di 250 caratteri di texts.ts.
import type { HelpId, HelpText } from './texts'

export const HELP_EN: Record<HelpId, HelpText> = {
  run: {
    title: 'Run',
    text: 'The button starts the acquisition and updates the view continuously. Use it to follow a signal that changes over time. With a 1 kHz PWM signal, you see the shape repeat every cycle.',
  },
  stop: {
    title: 'Stop',
    text: 'The button stops the acquisition and keeps the last shape on screen. Use it to examine the signal without it moving. With an edge too fast to read, stop on it and observe it calmly.',
  },
  single: {
    title: 'Single acquisition',
    text: 'The button captures one acquisition when the trigger fires, then stops. Use it to record an event that happens only once, such as a motor starting. Set the trigger to the right edge first.',
  },
  triggerMode: {
    title: 'Trigger mode',
    text: 'The mode decides when the view updates. Auto shows the signal even without events. Normal waits for an event that crosses the level, so the shape stays still.',
  },
  triggerSource: {
    title: 'Trigger source',
    text: 'Selects the channel that fires the trigger. The trigger is the point where each acquisition starts. With a signal on channel 1 and a reference on channel 2, pick channel 1 to align the shape.',
  },
  triggerKind: {
    title: 'Trigger type',
    text: 'Selects what fires the trigger, for example an edge or a pulse width. The edge is the simplest choice. With a 1 kHz square wave, edge triggering shows each period at the same point.',
  },
  triggerLevel: {
    title: 'Trigger level',
    text: 'Threshold voltage the signal must cross to fire the trigger. You can also drag the triangle on the right side of the display.',
  },
  triggerSlope: {
    title: 'Trigger slope',
    text: 'Selects whether the trigger fires on the rising edge, the falling edge or both. The rising edge is the transition from low to high. With a 1 kHz square wave, it gives a stable image of each period.',
  },
  hysteresis: {
    title: 'Hysteresis',
    text: 'Margin around the trigger level. The signal must move beyond it before a new trigger is accepted, so noise does not cause multiple triggers.',
  },
  holdoff: {
    title: 'Holdoff',
    text: 'Minimum time between two consecutive triggers. Useful with burst signals or many closely spaced edges, to always lock on the same point.',
  },
  pretrigger: {
    title: 'Pre-trigger',
    text: 'Shows part of the signal before the trigger, so you see what happens before the event. Use it to find the cause of a fault or a glitch. With an abnormal 10 ms pulse, it shows the stretch before it.',
  },
  timePerDiv: {
    title: 'Time base',
    text: 'Sets how much time one screen division represents. A small value shows fast details, a large value shows a long interval. With a 1 kHz PWM signal, 200 µs/div shows two periods.',
  },
  horizontalOffset: {
    title: 'Horizontal offset',
    text: 'Shifts the view in time, to the right or to the left. Use it to see a part of the signal that is off screen. With an event 5 ms after the trigger, shift the view by 5 ms to bring it back to center.',
  },
  channelEnable: {
    title: 'Enable channel',
    text: 'Shows or hides the trace of a channel. Hiding a channel frees the screen when you look at the other one. With two connected signals, enable only channel 1 to observe it better.',
  },
  voltsPerDiv: {
    title: 'Volts per division',
    text: 'Sets how much voltage one vertical division represents. A small value enlarges weak signals, a large value fits strong signals on screen. With a 0 to 3 V signal, 1 V/div shows it fully.',
  },
  verticalOffset: {
    title: 'Vertical offset',
    text: 'Moves the zero position of the channel up or down. Use it to separate two overlapping traces. With two identical signals, shift one by two divisions to see both.',
  },
  probe: {
    title: 'Probe',
    text: 'Sets the attenuation ratio of the connected probe, for example 1x or 10x. The instrument corrects the displayed volts using this value. With a 10x probe, choose 10x to read the real circuit voltage.',
  },
  coupling: {
    title: 'Coupling',
    text: 'Selects whether the signal passes with its DC component or without it (AC). In DC you see the whole signal, in AC only the variations. With 5 V and small ripple, use AC to see them.',
  },
  cursors: {
    title: 'Cursors',
    text: 'Shows two lines you can move on the graph to read times and voltages precisely. Use them to measure the distance between two points. With a 1 kHz wave, place them on two rising edges to read the 1 ms period.',
  },
  view: {
    title: 'View',
    text: 'The selector changes what the graph shows. The time view shows the wave, the spectrum shows the frequencies present. With a 1 kHz signal, the spectrum shows a peak near 1 kHz.',
  },
  fftWindow: {
    title: 'FFT window',
    text: 'Selects the window that attenuates the signal edges before frequency analysis. Each window balances frequency and amplitude accuracy. With a pure signal, the Flat-top window gives the most accurate peak amplitude.',
  },
  persistence: {
    title: 'Persistence',
    text: 'Lets previous traces fade out instead of disappearing at once. Shows how much a signal varies over time. With a signal that rings on its edges, persistence shows the band of variation.',
  },
  measurements: {
    title: 'Measurements',
    text: 'Shows values calculated from the signal, such as maximum voltage, frequency and rise time. They update on every acquisition. With a 1 kHz square wave, the panel shows a frequency close to 1 kHz.',
  },
  vpp: {
    title: 'Peak-to-peak voltage',
    text: 'Shows the difference between the maximum and minimum value of the signal. Use it to know the total swing of the signal. With a wave from 0 to 5 V, the measurement reads about 5 V.',
  },
  vrms: {
    title: 'RMS voltage',
    text: 'Shows the RMS value of the signal, the DC voltage that produces the same heating. Use it to evaluate the power of an alternating signal. With a sine of 1 V peak, the RMS value is about 0.71 V.',
  },
  vacRms: {
    title: 'Vac rms',
    text: 'RMS value of the alternating part only, without the DC component.',
  },
  frequency: {
    title: 'Frequency',
    text: 'Shows how many times per second the signal repeats. Use it to check the frequency of an oscillator or a PWM. With a PWM set to 1 kHz, the measurement shows a value close to 1 kHz.',
  },
  duty: {
    title: 'Duty cycle',
    text: 'Percentage of the period during which the signal stays high.',
  },
  riseTime: {
    title: 'Rise time',
    text: 'Time the edge takes to go from 10% to 90% of the amplitude. The shorter it is, the steeper the edge.',
  },
  thd: {
    title: 'Harmonic distortion',
    text: 'Shows how much the signal deviates from a pure sine, as a percentage. Use it to check the quality of a wave generated by an amplifier or an oscillator. With a clean sine, the value is close to zero.',
  },
  phase: {
    title: 'Phase',
    text: 'Shows the phase shift between two signals with the same frequency, in degrees. Use it to tell whether one signal lags the other. With two 1 kHz waves shifted by a quarter period, it shows about 90 degrees.',
  },
  sampleRate: {
    title: 'Sample rate',
    text: 'Shows how many samples per second the instrument takes from the signal. More samples give finer detail, but fill memory faster. With 1 MS/s, a 1 kHz wave has about a thousand samples per cycle.',
  },
  recordLength: {
    title: 'Record length',
    text: 'Shows how many samples are recorded in one acquisition. A long record covers more time with more detail, but needs more processing. The instrument calculates it from the time base and the sample rate.',
  },
  math: {
    title: 'Math functions',
    text: 'Computes a trace M from CH1 and CH2: sum, difference or product. Derivative and integral use only CH1. Trace M appears on screen as one more channel.',
  },
  filter: {
    title: 'Filter',
    text: 'Applies a low-pass, high-pass or band-pass filter to the channel. The low-pass attenuates fast noise, the high-pass the DC component. With a noisy signal, lower the cutoff frequency.',
  },
  classify: {
    title: 'Signal classification',
    text: 'Recognizes the signal type on the channel, for example DC, noise, clock, PWM, serial or burst. It also suggests suitable decoders and an estimate of the speed. With a serial line, it suggests the decoder to try.',
  },
  recording: {
    title: 'Recording',
    text: 'Saves the acquired frames to a CSV, NPY or WAV file in the h6022-recordings folder. You can stop it by hand or after a number of frames or seconds. While recording, the top bar shows REC.',
  },
  decoder: {
    title: 'Decoder',
    text: 'Decodes a UART, I2C or CAN serial line and shows the bytes below the display. Choose the channel, the threshold and the line speed. With the wrong protocol, the annotations are wrong or missing.',
  },
  mcp: {
    title: 'MCP server',
    text: 'The MCP server starts with the app on port 6022. A client such as Claude Code uses it to read measurements, acquire and decode. With the green dot, the server is reachable.',
  },
  device: {
    title: 'Device',
    text: 'Shows the data source: Hantek 6022BE over USB or the simulator. The menu shows firmware and calibration. If the device does not appear, reconnect the USB cable.',
  },
  trend: {
    title: 'Measurement trend',
    text: 'Shows how a measurement changes over time, acquisition after acquisition. Use it to notice a slow drift that is not visible in the single waveform. With the supply voltage slowly dropping, the trend shows the decline.',
  },
  explain: {
    title: 'Explain',
    text: 'Turns these explanations on or off. With the mode active, hover over a control to read what it does. The i key does the same.',
  },
  status: {
    title: 'Acquisition status',
    text: 'Shows whether the acquisition is running, stopped or waiting for a trigger. In Normal mode, “Waiting” means no event meets the trigger condition.',
  },
  scope: {
    title: 'Display',
    text: 'Shows the traces as a function of time. Hover to read time and voltages at that point. Drag the markers on the left to move the channels, and the T triangle on the right for the trigger level.',
  },
  panelTabs: {
    title: 'Panel tabs',
    text: 'Controls groups channels, time base, trigger and display. Decoder sets up protocol decoding. Tools contains recording, classification, math and filter.',
  },
  logFrequency: {
    title: 'Logarithmic axis',
    text: 'Shows the frequency axis of the spectrum on a logarithmic scale. Use it to see low and high harmonics together, for example from 50 Hz to 50 kHz.',
  },
  pulsePolarity: {
    title: 'Pulse polarity',
    text: 'Selects whether to measure positive pulses, above the level, or negative pulses, below the level.',
  },
  pulseWidth: {
    title: 'Pulse width',
    text: 'The trigger fires when a pulse is shorter, longer, inside or outside the set limits. Use it to catch an abnormal pulse in a PWM train.',
  },
  runt: {
    title: 'Runt trigger',
    text: 'Fires on a pulse that crosses the low threshold but does not reach the high one. Use it to find weak pulses in a digital signal, for example from an unstable power supply.',
  },
  windowTrigger: {
    title: 'Window trigger',
    text: 'Fires when the signal enters or leaves the band between the low and high thresholds. Use it to monitor a voltage that must stay within a range.',
  },
  timeoutTrigger: {
    title: 'Timeout trigger',
    text: 'Fires when the signal does not show the chosen edge within the set time. Use it to catch a clock that stops or a bus that stays idle.',
  },
  patternTrigger: {
    title: 'Pattern trigger',
    text: 'Fires when both channels take the chosen combination of high and low levels. The value “any” excludes the channel from the comparison.',
  },
  logicTrigger: {
    title: 'Logic trigger',
    text: 'Each channel has a condition: edge, high, low or ignore. AND fires when all are true, OR when one is enough. Use it to catch an event that can appear on either channel.',
  },
  otherLevel: {
    title: 'Other channel level',
    text: 'Threshold of the channel that is not the source. Each channel thus has its own level in the logic and A → B triggers.',
  },
  sequenceTrigger: {
    title: 'Trigger A → B',
    text: 'Edge A on the source channel arms the trigger. The frame fires on the first edge B on the other channel, which becomes time zero. Use it to measure a cause and its effect.',
  },
  sequenceDelay: {
    title: 'B delay',
    text: 'Minimum time between edge A and edge B. B edges that arrive earlier are ignored. With 0, the first B edge after A is used.',
  },
  vmax: {
    title: 'Vmax',
    text: 'Highest voltage in the visible window. Includes peaks and noise.',
  },
  vmin: {
    title: 'Vmin',
    text: 'Lowest voltage in the visible window. Includes peaks and noise.',
  },
  vmean: {
    title: 'Vmean',
    text: 'Average voltage in the window. Corresponds to the DC component of the signal.',
  },
  vtop: {
    title: 'Vtop',
    text: 'Stable high level of the signal, calculated from the voltage histogram. Excludes overshoot and peaks, unlike Vmax.',
  },
  vbase: {
    title: 'Vbase',
    text: 'Stable low level of the signal, calculated from the voltage histogram. Excludes undershoot and peaks, unlike Vmin.',
  },
  vamp: {
    title: 'Amplitude',
    text: 'Difference between Vtop and Vbase. For a square wave, it is the real swing between the two levels, without peaks.',
  },
  fallTime: {
    title: 'Fall time',
    text: 'Time the signal takes to fall from 90% to 10% of the amplitude. Use it to evaluate the speed of a digital output.',
  },
  overshoot: {
    title: 'Overshoot',
    text: 'How much the signal exceeds Vtop after an edge, as a percentage of the amplitude. A high value indicates resonance or a mismatched load.',
  },
  period: {
    title: 'Period',
    text: 'Duration of one complete cycle of the signal, the inverse of the frequency. Shows “—” if the window does not contain a whole period.',
  },
  pulseWidthMeasure: {
    title: 'Pulse width',
    text: 'Duration of the high (+) and low (−) part of a cycle, measured at half amplitude.',
  },
  edges: {
    title: 'Edges',
    text: 'Number of rising and falling edges in the visible window.',
  },
  delay: {
    title: 'CH1–CH2 delay',
    text: 'Time between an edge of CH1 and the corresponding edge of CH2. Use it to measure the latency of a circuit.',
  },
  decoderProtocol: {
    title: 'Protocol',
    text: 'Selects the protocol to decode, for example UART, I2C or CAN. The parameters below change depending on the protocol.',
  },
  decoderChannel: {
    title: 'Decoder channel',
    text: 'Selects the channel that carries the signal to decode.',
  },
  decoderThreshold: {
    title: 'Decoder threshold',
    text: 'Voltage that separates logic 0 from logic 1. With the automatic threshold, the decoder uses the midpoint between the high and low levels of the signal.',
  },
  engine: {
    title: 'Engine',
    text: 'Data source and number of frames per second shown. The measured sample rate shows the samples that actually arrive from the device.',
  },
}
