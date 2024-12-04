export enum PageType {
  LOGS = "logs",
  STREAM_EXPLORER = "stream_explorer",
  TRACE_EXPLORER = "trace_explorer",
}

export interface ActivationState {
  isSearchTab: boolean;
  isStreamExplorer: boolean;
  isTraceExplorer: boolean;
  isStreamChanged: boolean;
}
