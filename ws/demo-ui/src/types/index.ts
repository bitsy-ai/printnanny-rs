export * from "./alerts"
export * from "./events"


export interface JanusMedia {
  age_ms: number,
  label: String,
  mid: String,
  type: String,
  codec: String
}
export interface JanusStream {
  description: String,
  enabled: boolean,
  id: number,
  media: Array<JanusMedia>
}
