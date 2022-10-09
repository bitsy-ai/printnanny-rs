export * from "./alerts"
export * from "./events"


export interface JanusMedia {
  age_ms: number,
  label: string,
  mid: string,
  type: string,
  codec: string
}
export interface JanusStream {
  description: string,
  enabled: boolean,
  id: number,
  media: Array<JanusMedia>
}
