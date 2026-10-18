// Spiegazioni dei controlli in francese. Stesse chiavi e stesso limite di 250 caratteri di texts.ts.
import type { HelpId, HelpText } from './texts'

export const HELP_FR: Record<HelpId, HelpText> = {
  run: {
    title: 'Démarrer',
    text: 'Le bouton lance l’acquisition et met la vue à jour en continu. Sert à suivre un signal qui change dans le temps. Avec un signal PWM à 1 kHz, on voit la forme se répéter à chaque cycle.',
  },
  stop: {
    title: 'Arrêter',
    text: 'Le bouton arrête l’acquisition et laisse la dernière forme à l’écran. Sert à examiner le signal sans qu’il bouge. Avec un front trop rapide pour être lu, figez-le et observez-le calmement.',
  },
  single: {
    title: 'Acquisition unique',
    text: 'Le bouton capture une seule acquisition quand le trigger se déclenche, puis s’arrête. Sert à enregistrer un événement qui ne se produit qu’une fois, comme la mise en marche d’un moteur. Règle d’abord le trigger sur le bon front.',
  },
  triggerMode: {
    title: 'Mode trigger',
    text: 'Le mode décide quand la vue se met à jour. Automatique affiche le signal même sans événements. Normal attend un événement qui dépasse le niveau, ainsi la forme reste immobile.',
  },
  triggerSource: {
    title: 'Source trigger',
    text: 'Choisit le canal qui déclenche le trigger. Le trigger est le point à partir duquel commence chaque acquisition. Avec un signal sur CH1 et une référence sur CH2, choisis CH1 pour aligner la forme.',
  },
  triggerKind: {
    title: 'Type de trigger',
    text: 'Choisit ce qui déclenche le trigger, par exemple un front ou une largeur d’impulsion. Le front est le choix le plus simple. Avec une onde carrée à 1 kHz, le trigger sur front montre chaque période au même point.',
  },
  triggerLevel: {
    title: 'Niveau trigger',
    text: 'Tension de seuil que le signal doit franchir pour déclencher le trigger. Tu peux aussi faire glisser le triangle à droite de l’écran.',
  },
  triggerSlope: {
    title: 'Pente trigger',
    text: 'Choisit si le trigger se déclenche sur le front montant, sur le front descendant ou sur les deux. Le front montant est le passage de bas en haut. Avec une onde carrée à 1 kHz, le front montant donne une image stable de chaque période.',
  },
  hysteresis: {
    title: 'Hystérésis',
    text: 'Marge autour du niveau de trigger. Le signal doit la dépasser avant qu’un nouveau trigger soit accepté, pour que le bruit ne cause pas de déclenchements multiples.',
  },
  holdoff: {
    title: 'Attente trigger',
    text: 'Temps minimal entre deux triggers consécutifs. Utile avec des signaux en rafale ou avec beaucoup de fronts rapprochés, pour toujours accrocher le même point.',
  },
  pretrigger: {
    title: 'Pré-trigger',
    text: 'Affiche une partie du signal avant le trigger, pour voir ce qui se passe avant l’événement. Sert quand on cherche la cause d’une panne ou d’une perturbation. Avec une impulsion anormale à 10 ms, le pré-trigger montre la portion qui la précède.',
  },
  timePerDiv: {
    title: 'Base de temps',
    text: 'Règle le temps représenté par une division de l’écran. Une petite valeur montre des détails rapides, une grande valeur montre un long intervalle. Avec un signal PWM à 1 kHz, 200 µs/div montre deux périodes.',
  },
  horizontalOffset: {
    title: 'Décalage horizontal',
    text: 'Déplace la vue dans le temps, vers la droite ou vers la gauche. Sert à voir une partie du signal qui n’apparaît pas à l’écran. Avec un événement 5 ms après le trigger, décale la vue de 5 ms pour le ramener au centre.',
  },
  channelEnable: {
    title: 'Activer canal',
    text: 'Affiche ou masque la trace d’un canal. Masquer un canal libère l’écran quand on observe l’autre. Avec deux signaux connectés, active seulement CH1 pour mieux l’observer.',
  },
  voltsPerDiv: {
    title: 'Volts par division',
    text: 'Règle la tension représentée par une division verticale. Une petite valeur agrandit les signaux faibles, une grande valeur fait tenir les signaux forts à l’écran. Avec un signal de 0 à 3 V, 1 V/div l’affiche en entier.',
  },
  verticalOffset: {
    title: 'Décalage vertical',
    text: 'Déplace vers le haut ou vers le bas la position du zéro du canal. Sert à séparer deux traces qui se superposent. Avec deux signaux identiques, décale l’un de deux divisions pour les voir tous les deux.',
  },
  probe: {
    title: 'Sonde',
    text: 'Indique le rapport de la sonde branchée, par exemple 1x ou 10x. L’instrument corrige les volts affichés selon cette valeur. Avec une sonde 10x, choisis 10x pour lire la tension réelle du circuit.',
  },
  coupling: {
    title: 'Couplage',
    text: 'Choisit si le signal passe avec la composante continue (DC) ou sans (AC). En DC on voit tout le signal, en AC seulement les variations. Avec 5 V et de petites ondulations, utilise AC pour les voir.',
  },
  cursors: {
    title: 'Curseurs',
    text: 'Affiche deux lignes que tu peux déplacer sur le graphique pour lire précisément temps et tensions. Sert à mesurer la distance entre deux points. Avec une onde à 1 kHz, place les curseurs sur deux fronts montants pour lire la période de 1 ms.',
  },
  view: {
    title: 'Vue',
    text: 'Le sélecteur change ce que montre le graphique. La vue temporelle montre l’onde, le spectre les fréquences présentes. Avec un signal à 1 kHz, le spectre montre un pic proche de 1 kHz.',
  },
  fftWindow: {
    title: 'Fenêtre FFT',
    text: 'Choisit la fenêtre qui atténue les bords du signal avant l’analyse des fréquences. Chaque fenêtre équilibre précision en fréquence et en amplitude. Avec un signal pur, la fenêtre Flat-top donne l’amplitude du pic la plus juste.',
  },
  persistence: {
    title: 'Persistance',
    text: 'Laisse s’estomper les traces précédentes au lieu de les faire disparaître d’un coup. Montre de combien un signal varie dans le temps. Avec un signal qui oscille un peu sur les fronts, la persistance montre la bande de variation.',
  },
  measurements: {
    title: 'Mesures',
    text: 'Affiche des valeurs calculées à partir du signal, comme la tension maximale, la fréquence et le temps de montée. Elles se mettent à jour à chaque acquisition. Avec une onde carrée à 1 kHz, le panneau indique une fréquence proche de 1 kHz.',
  },
  vpp: {
    title: 'Tension crête à crête',
    text: 'Indique la différence entre la valeur maximale et la valeur minimale du signal. Sert à savoir de combien le signal oscille au total. Avec une onde qui va de 0 à 5 V, la mesure affiche environ 5 V.',
  },
  vrms: {
    title: 'Tension efficace',
    text: 'Indique la valeur efficace du signal, c’est-à-dire la tension continue qui produit le même échauffement. Sert à évaluer la puissance d’un signal alternatif. Avec une sinusoïde de crête 1 V, la valeur efficace est d’environ 0,71 V.',
  },
  vacRms: {
    title: 'Vac rms',
    text: 'Valeur efficace de la seule partie alternative, sans la composante continue.',
  },
  frequency: {
    title: 'Fréquence',
    text: 'Indique combien de fois par seconde le signal se répète. Sert à vérifier la fréquence d’un oscillateur ou d’un PWM. Avec un PWM réglé à 1 kHz, la mesure affiche une valeur proche de 1 kHz.',
  },
  duty: {
    title: 'Rapport cyclique',
    text: 'Pourcentage de la période pendant laquelle le signal reste au niveau haut.',
  },
  riseTime: {
    title: 'Temps de montée',
    text: 'Temps que met le front pour passer de 10 % à 90 % de l’amplitude. Plus il est court, plus le front est raide.',
  },
  thd: {
    title: 'Distorsion harmonique',
    text: 'Indique de combien le signal s’écarte d’une sinusoïde pure, en pourcentage. Sert à vérifier la qualité d’une onde produite par un amplificateur ou un oscillateur. Avec une sinusoïde propre, la valeur est proche de zéro.',
  },
  phase: {
    title: 'Phase',
    text: 'Indique le déphasage entre deux signaux de même fréquence, en degrés. Sert à savoir si un signal est en retard sur l’autre. Avec deux ondes à 1 kHz déphasées d’un quart de période, la mesure affiche environ 90 degrés.',
  },
  sampleRate: {
    title: 'Fréquence d’échantillonnage',
    text: 'Indique combien d’échantillons par seconde l’instrument prélève sur le signal. Plus il y en a, plus les détails sont fins, mais la mémoire se remplit plus vite. Avec 1 MS/s, une onde à 1 kHz a environ mille échantillons par cycle.',
  },
  recordLength: {
    title: 'Longueur de l’enregistrement',
    text: 'Indique combien d’échantillons sont enregistrés par acquisition. Un enregistrement long couvre plus de temps avec plus de détails, mais demande plus de calcul. L’instrument le calcule à partir de la base de temps.',
  },
  math: {
    title: 'Fonctions mathématiques',
    text: 'Calcule une trace M à partir de CH1 et CH2 : somme, différence ou produit. La dérivée et l’intégrale utilisent seulement CH1. La trace M apparaît à l’écran comme un canal supplémentaire.',
  },
  filter: {
    title: 'Filtre',
    text: 'Applique au canal un filtre passe-bas, passe-haut ou passe-bande. Le passe-bas atténue le bruit rapide, le passe-haut la composante continue. Avec un signal bruité, abaisse la fréquence de coupure.',
  },
  classify: {
    title: 'Classification du signal',
    text: 'Reconnaît le type de signal sur le canal, par exemple continu, bruit, horloge, PWM, série ou rafale. Indique aussi les décodeurs les mieux adaptés et une estimation de la vitesse. Avec une ligne série, la classe suggère le décodeur à essayer.',
  },
  recording: {
    title: 'Enregistrement',
    text: 'Enregistre les trames acquises dans un fichier CSV, NPY ou WAV, dans le dossier h6022-recordings. Tu peux l’arrêter à la main ou après un nombre de trames ou de secondes. Pendant l’enregistrement, la barre du haut affiche REC.',
  },
  decoder: {
    title: 'Décodeur',
    text: 'Décode une ligne série UART, I2C ou CAN et affiche les octets sous l’écran. Choisis le canal, le seuil et la vitesse de la ligne. Avec un mauvais protocole, les annotations sont fausses ou absentes.',
  },
  mcp: {
    title: 'Serveur MCP',
    text: 'Le serveur MCP démarre avec l’application sur le port 6022. Un client comme Claude Code l’utilise pour lire des mesures, acquérir et décoder. Avec le point vert, le serveur est joignable.',
  },
  device: {
    title: 'Appareil',
    text: 'Affiche la source des données : Hantek 6022BE via USB ou le simulateur. Le menu indique le firmware et l’étalonnage. Si l’appareil n’apparaît pas, rebranche le câble USB.',
  },
  trend: {
    title: 'Tendance de la mesure',
    text: 'Montre comment une mesure change dans le temps, acquisition après acquisition. Sert à repérer une lente dérive qui ne se voit pas dans une forme d’onde isolée. Si la tension d’alimentation baisse doucement, la tendance montre la descente.',
  },
  explain: {
    title: 'Explications',
    text: 'Active ou désactive ces explications. En mode actif, passe le pointeur sur un contrôle pour lire à quoi il sert. La touche i fait la même chose.',
  },
  status: {
    title: 'État de l’acquisition',
    text: 'Indique si l’acquisition est en cours, arrêtée ou en attente d’un trigger. En mode Normal, « En attente » signifie qu’aucun événement ne satisfait la condition de trigger.',
  },
  scope: {
    title: 'Écran',
    text: 'Affiche les traces en fonction du temps. Passe le pointeur pour lire temps et tensions à cet endroit. Fais glisser les marqueurs à gauche pour déplacer les canaux et le triangle T à droite pour le niveau de trigger.',
  },
  panelTabs: {
    title: 'Onglets du panneau',
    text: 'Contrôles regroupe canaux, base de temps, trigger et écran. Décodeur règle le décodage des protocoles. Outils contient enregistrement, classification, mathématiques et filtre.',
  },
  logFrequency: {
    title: 'Axe logarithmique',
    text: 'Affiche l’axe des fréquences du spectre en échelle logarithmique. Sert à voir ensemble les harmoniques basses et hautes, par exemple de 50 Hz à 50 kHz.',
  },
  pulsePolarity: {
    title: 'Polarité d’impulsion',
    text: 'Choisit de mesurer les impulsions positives, c’est-à-dire au-dessus du niveau, ou les négatives, en dessous du niveau.',
  },
  pulseWidth: {
    title: 'Largeur d’impulsion',
    text: 'Le trigger se déclenche quand une impulsion dure moins, plus, à l’intérieur ou à l’extérieur des limites réglées. Sert à capturer une impulsion anormale dans un train PWM.',
  },
  runt: {
    title: 'Trigger runt',
    text: 'Se déclenche sur une impulsion qui dépasse le seuil bas sans atteindre le seuil haut. Sert à trouver des impulsions faibles dans un signal numérique, par exemple pour une alimentation instable.',
  },
  windowTrigger: {
    title: 'Trigger à fenêtre',
    text: 'Se déclenche quand le signal entre ou sort de la plage comprise entre le seuil bas et le seuil haut. Sert à surveiller une tension qui doit rester dans un intervalle.',
  },
  timeoutTrigger: {
    title: 'Trigger timeout',
    text: 'Se déclenche quand le signal ne présente pas le front choisi pendant le temps réglé. Sert à capturer une horloge qui s’arrête ou un bus qui reste inactif.',
  },
  patternTrigger: {
    title: 'Trigger à motif',
    text: 'Se déclenche quand les deux canaux prennent la combinaison de niveaux haut et bas choisie. La valeur « quelconque » exclut le canal de la comparaison.',
  },
  logicTrigger: {
    title: 'Trigger logique',
    text: 'Chaque canal a une condition : front, haut, bas ou ignorer. AND se déclenche quand toutes sont vraies, OR quand une seule suffit. Sert à capturer un événement qui peut apparaître sur l’un des deux canaux.',
  },
  otherLevel: {
    title: 'Niveau de l’autre canal',
    text: 'Seuil du canal qui n’est pas la source. Ainsi chaque canal a son propre niveau dans les triggers logique et A → B.',
  },
  sequenceTrigger: {
    title: 'Trigger A → B',
    text: 'Le front A sur le canal source arme le trigger. La trame se déclenche au premier front B sur l’autre canal, qui devient le temps zéro. Sert à mesurer une cause et son effet.',
  },
  sequenceDelay: {
    title: 'Délai B',
    text: 'Temps minimal entre le front A et le front B. Les fronts B qui arrivent plus tôt sont ignorés. Avec 0, c’est le premier front B après A qui compte.',
  },
  vmax: {
    title: 'Vmax',
    text: 'Tension la plus haute dans la fenêtre visible. Inclut pics et perturbations.',
  },
  vmin: {
    title: 'Vmin',
    text: 'Tension la plus basse dans la fenêtre visible. Inclut pics et perturbations.',
  },
  vmean: {
    title: 'Vmoyenne',
    text: 'Valeur moyenne de la tension dans la fenêtre. Correspond à la composante continue du signal.',
  },
  vtop: {
    title: 'Vtop',
    text: 'Niveau haut stable du signal, calculé à partir de l’histogramme des tensions. Exclut dépassements et pics, contrairement à Vmax.',
  },
  vbase: {
    title: 'Vbase',
    text: 'Niveau bas stable du signal, calculé à partir de l’histogramme des tensions. Exclut sous-dépassements et pics, contrairement à Vmin.',
  },
  vamp: {
    title: 'Amplitude',
    text: 'Différence entre Vtop et Vbase. Pour une onde carrée, c’est l’excursion réelle entre les deux niveaux, sans les pics.',
  },
  fallTime: {
    title: 'Temps de descente',
    text: 'Temps que met le signal à descendre de 90 % à 10 % de l’amplitude. Sert à évaluer la vitesse d’une sortie numérique.',
  },
  overshoot: {
    title: 'Overshoot',
    text: 'De combien le signal dépasse Vtop après un front, en pourcentage de l’amplitude. Une valeur élevée indique des résonances ou une charge mal adaptée.',
  },
  period: {
    title: 'Période',
    text: 'Durée d’un cycle complet du signal, c’est-à-dire l’inverse de la fréquence. Affiche « — » si la fenêtre ne contient pas une période entière.',
  },
  pulseWidthMeasure: {
    title: 'Largeur d’impulsion',
    text: 'Durée de la partie haute (+) et de la partie basse (−) d’un cycle, mesurée à mi-amplitude.',
  },
  edges: {
    title: 'Fronts',
    text: 'Nombre de fronts montants et descendants dans la fenêtre visible.',
  },
  delay: {
    title: 'Retard CH1–CH2',
    text: 'Temps entre un front de CH1 et le front correspondant de CH2. Sert à mesurer la latence d’un circuit.',
  },
  decoderProtocol: {
    title: 'Protocole',
    text: 'Choisit le protocole à décoder, par exemple UART, I2C ou CAN. Les paramètres ci-dessous changent selon le protocole.',
  },
  decoderChannel: {
    title: 'Canal du décodeur',
    text: 'Choisit le canal qui porte le signal à décoder.',
  },
  decoderThreshold: {
    title: 'Seuil du décodeur',
    text: 'Tension qui sépare le 0 du 1 logique. Avec le seuil automatique, le décodeur utilise le point médian entre les niveaux haut et bas du signal.',
  },
  engine: {
    title: 'Moteur',
    text: 'Source des données et nombre de trames par seconde affichés. La fréquence d’échantillonnage mesurée indique les échantillons qui arrivent vraiment de l’appareil.',
  },
}
