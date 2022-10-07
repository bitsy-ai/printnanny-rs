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
    Pending,
    Connected,
    ConnectionError
}