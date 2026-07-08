export type DataSection = {
  key: string;
  title: string;
  responseField: string;
  status: "ok" | "unsupported" | "restricted" | "error" | "idle";
  ok: boolean;
  raw: string;
  error?: string | null;
};

export type ActionDef = {
  id: string;
  name: string;
  confirm?: string | null;
};

export type KeyMetrics = {
  softwareVersion?: string | null;
  uptimeS?: number | null;
  downlinkMbps?: number | null;
  uplinkMbps?: number | null;
  latencyMs?: number | null;
  pingDropPercent?: number | null;
  obstructionPercent?: number | null;
  gpsSats?: number | null;
  currentlyObstructed?: boolean | null;
  boresightAzimuthDeg?: number | null;
  boresightElevationDeg?: number | null;
};

export type GpsInfo = {
  valid: boolean;
  sats: number;
  inhibited: boolean;
  noSatsAfterTtff: boolean;
};

export type OutageInfo = {
  cause: string;
  durationS: number;
};

export type ObstructionDetail = {
  fractionObstructedPercent: number;
  currentlyObstructed: boolean;
  timeObstructedS: number;
  avgProlongedObstructionDurationS?: number | null;
  avgProlongedObstructionIntervalS?: number | null;
  patchesValid: number;
};

export type LocationInfo = {
  lat?: number | null;
  lon?: number | null;
  altM?: number | null;
  source: string;
  accuracyM: number;
  horizontalSpeedMps: number;
  verticalSpeedMps: number;
};

export type DishOverview = {
  keyMetrics: KeyMetrics;
  alerts: string[];
  outage?: OutageInfo | null;
  disablementCode: string;
  gps: GpsInfo;
  obstruction?: ObstructionDetail | null;
  mobilityClass: string;
  softwareUpdateState: string;
  stowRequested: boolean;
  ethSpeedMbps: number;
  connectedRoutersCount: number;
  rebootReason: string;
  lastBootUnixS: number;
  utcOffsetS: number;
  countryCode: string;
  location?: LocationInfo | null;
  config?: DishConfigInfo | null;
};

export type OutageEvent = {
  cause: string;
  startUnixS: number;
  durationS: number;
};

export type DishHistory = {
  latencyMs: number[];
  pingDropPercent: number[];
  downlinkMbps: number[];
  uplinkMbps: number[];
  powerInW: number[];
  outages: OutageEvent[];
};

export type DishConfigInfo = {
  snowMeltMode: string;
  powerSaveMode: boolean;
  powerSaveStartMinutes: number;
  powerSaveDurationMinutes: number;
  levelDishMode: string;
  swupdateThreeDayDeferralEnabled: boolean;
  swupdateRebootHour: number;
};

export type DishConfigPatch = {
  snowMeltMode?: string;
  powerSaveMode?: boolean;
  powerSaveStartMinutes?: number;
  powerSaveDurationMinutes?: number;
  levelDishMode?: string;
  swupdateThreeDayDeferralEnabled?: boolean;
  swupdateRebootHour?: number;
};

export type StarlinkSnapshot = {
  dishAddress: string;
  fetchedAtUnixS: number;
  dishUp: boolean;
  overview?: DishOverview | null;
  sections: DataSection[];
  actions: ActionDef[];
};

export type ObstructionMap = {
  numRows: number;
  numCols: number;
  snr: number[];
  maxThetaDeg: number;
  referenceFrame: string;
};

export type WifiClientInfo = {
  name: string;
  macAddress: string;
  ipAddress: string;
  signalStrength: number;
  snr: number;
  associatedTimeS: number;
  iface: string;
};

export type WifiNetworkInfo = {
  band: string;
  ssid: string;
  hidden: boolean;
  disabled: boolean;
};

export type WifiConfigInfo = {
  channel2ghz: number;
  channel5ghz: number;
  dfsEnabled: boolean;
  secureDns: boolean;
  bypassMode: boolean;
  networks: WifiNetworkInfo[];
};

export type RebootEventInfo = {
  count: number;
  lastOccurredUnixS: number;
};

export type RouterOverview = {
  softwareVersion?: string | null;
  uptimeS?: number | null;
  pingLatencyMs: number;
  pingDropPercent: number;
  dishPingLatencyMs: number;
  dishPingDropPercent: number;
  alerts: string[];
  clients: WifiClientInfo[];
  ssids: string[];
  setupComplete: boolean;
  bypassMode: boolean;
  rebootFromUpdate?: RebootEventInfo | null;
  config?: WifiConfigInfo | null;
};

export type RouterHistory = {
  pingLatencyMs: number[];
  pingDropPercent: number[];
};

export type NetworkPatch = {
  band: string;
  ssid?: string;
  password?: string;
  hidden?: boolean;
  disabled?: boolean;
};

export type WifiConfigPatch = {
  channel2ghz?: number;
  channel5ghz?: number;
  dfsEnabled?: boolean;
  secureDns?: boolean;
  bypassMode?: boolean;
  network?: NetworkPatch;
};

export type RouterSnapshot = {
  routerAddress: string;
  fetchedAtUnixS: number;
  routerUp: boolean;
  overview?: RouterOverview | null;
  sections: DataSection[];
  actions: ActionDef[];
};

export type ActionExecutionResult = {
  id: string;
  ok: boolean;
  message: string;
};

export type TabKey = "overview" | "dish" | "router" | "diagnostics" | "settings";
