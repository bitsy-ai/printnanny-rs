import type { FunctionalComponent, HTMLAttributes, VNodeProps } from "vue";

export interface JanusMedia {
    age_ms: number;
    codec: string;
    label: string;
    mid: string;
    mindex: number;
    port: number;
    pt: number;
    rtpmap: string;
    type: string;
}

export interface JanusStreamMetadata {
    path: string;
}
export interface JanusStream {
    description: string;
    enabled: boolean;
    id: number;
    media: Array<JanusMedia>;
    metadata: JanusStreamMetadata,
    name: string;
    type: string;
    viewers: number
}


export enum NatsSubjectPattern {
    DataframeRow = "pi.qc.df",
    StreamRequest = "pi.qc.stream"
}

export interface QcDataframeRow {
    "detection_boxes_x0": number;
    "detection_boxes_y0": number;
    "detection_boxes_x1": number;
    "detection_boxes_y1": number;
    "detection_classes": number;
    "detection_scores": number;
    "ts": number;
}

export enum ConnectionStatus {
    ConnectionNotStarted,
    ConnectionLoading,
    ConnectionReady,
    ConnectionStreamLoading,
    ConnectionStreamReady,
    ConnectionError
}

export interface DetectionAlert {
    id: string;
    color: string;
    header: string;
    description: string;
    icon: FunctionalComponent<HTMLAttributes & VNodeProps>;
}

export enum NatsQcCommand {
    Start = "Start",
    Stop = "Stop"
}

export interface NatsQcStreamRequest {
    subject: string;
    janus_stream: JanusStream;
    command: NatsQcCommand;
}
