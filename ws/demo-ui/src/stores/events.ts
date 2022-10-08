import { defineStore, acceptHMRUpdate } from "pinia";
import { toRaw } from "vue";
import { connect, JSONCodec, credsAuthenticator, toJsMsg } from "nats.ws";
import type { NatsConnection, Subscription } from "nats.ws";
import Janode from "janode";
import StreamingPlugin from "janode/plugins/streaming";
import { ArrowDownIcon, ArrowUpIcon, CheckIcon, ExclamationTriangleIcon } from '@heroicons/vue/20/solid'

import type { QcDataframeRow, UiAlert } from "@types";
import { ConnectionStatus, type JanusMedia, type JanusStream, type DetectionAlert } from "@/types";
import { handleError } from "@/utils";

function getNatsURI() {
    const hostname = window.location.hostname;
    const uri = `ws://${hostname}:${import.meta.env.VITE_PRINTNANNY_EDGE_NATS_WS_PORT}`;
    console.log(`Connecting to NATS server: ${uri}`)
    return uri
}

function getJanusUri() {
    const hostname = window.location.hostname;
    const uri = `ws://${hostname}:${import.meta.env.VITE_PRINTNANNY_EDGE_JANUS_WS_PORT}`;
    console.log(`Connecting to Janus signaling websocket: ${uri}`);
    return uri
}

// returns true if num truthy elements / total elements >= threshold
function atLeast(arr: Array<boolean>, threshold: number): boolean {
    return arr.filter(el => el === true).length / arr.length >= threshold
}

const RTCPeerConnection = window.RTCPeerConnection.bind(window);

export const useEventStore = defineStore({
    id: "events",
    state: () => ({
        df: [] as Array<QcDataframeRow>,
        natsConnection: undefined as NatsConnection | undefined,
        janusWsConnection: undefined as undefined | Janode.Connection,
        janusSession: undefined as undefined | any,
        janusPeerConnection: undefined as undefined | RTCPeerConnection,
        janusStreamingPluginHandle: undefined as undefined | any,
        status: ConnectionStatus.ConnectionNotStarted as ConnectionStatus,
        alerts: [] as Array<UiAlert>,
        streamList: [] as Array<JanusStream>,
        selectedStream: undefined as undefined | JanusStream,
        detectionAlerts: [
            {
                id: "Example: Calibration Alert", icon: ExclamationTriangleIcon, color: "indigo", description: "If PrintNanny doesn't recognize your 3D printer, you'll see calibration recommendations."

            } as DetectionAlert,
            {
                id: "Example: Failure Alert", icon: ExclamationTriangleIcon, color: "red", description: "When a print job is failing, PrintNanny will notify you."

            } as DetectionAlert
        ] as Array<DetectionAlert>
    }),
    getters: {
        meter_x(state): Array<number> {
            return state.df.map(el => el.ts)
        },
        meter_y_nozzle_mean(state): Array<number> {
            return state.df.map(el => el.nozzle__mean)
        },
        meter_y_nozzle_std: (state) => state.df.map(el => el.nozzle__std),

        meter_y_print_mean: (state) => state.df.map(el => el.print__mean),
        meter_y_print_std: (state) => state.df.map(el => el.print__std),

        meter_y_raft_mean: (state) => state.df.map(el => el.raft__mean),
        meter_y_raft_std: (state) => state.df.map(el => el.raft__std),

        meter_y_adhesion_mean: (state) => state.df.map(el => el.adhesion__mean),
        meter_y_adhesion_std: (state) => state.df.map(el => el.adhesion__std),

        meter_y_spaghetti_mean: (state) => state.df.map(el => el.spaghetti__mean),
        meter_y_spaghetti_std: (state) => state.df.map(el => el.spaghetti__std),

    },
    actions: {

        async connectNats(): Promise<boolean> {
            // create nats connection if not initialized
            if (this.natsConnection === undefined) {
                const servers = [getNatsURI()];
                const connectOptions = {
                    servers,
                    debug: false,
                };

                if (import.meta.env.VITE_PRINTNANNY_DEBUG == true) {
                    connectOptions.debug = true;
                }
                const natsConnection = await connect(connectOptions).catch((e: Error) => handleError("Failed to connect to NATS server", e));
                if (natsConnection) {
                    console.log(`Initialized NATs connection to ${servers}`);
                    this.$patch({ natsConnection });
                    await this.subscribeQcDataframes();
                    return true
                }
                return false
            } else {
                return true
            }
        },

        async connectJanus(): Promise<boolean> {
            const janusUri = getJanusUri();
            const connectOpts = {
                is_admin: false,
                address: {
                    url: janusUri,
                },
            };
            const janusWsConnection: Janode.Connection = await Janode.connect(connectOpts).catch((e: Error) => handleError("Janus websocket connection failed", e));
            console.log("Got janusWsConnection", janusWsConnection);
            const janusSession = await janusWsConnection.create().catch((e: Error) => handleError("Failed to create Janus websocket session ", e));
            const janusStreamingPluginHandle = await janusSession.attach(StreamingPlugin)
                .catch((e: Error) => handleError("Failed to create Janus streaming handle", e));
            const streamListRes = await janusStreamingPluginHandle.list();
            console.log("Found streamlist", streamListRes);
            // get detailed info from streamlist
            const streamList = await Promise.all(streamListRes.list.map(async (stream: any) => {
                const res = await janusStreamingPluginHandle.info({ id: stream.id });
                return res
            }));
            console.log("Fetched detailed stream info", streamList);


            this.$patch({
                streamList,
                janusWsConnection,
                janusSession,
                janusStreamingPluginHandle,
            });

            janusStreamingPluginHandle.once(Janode.EVENT.HANDLE_DETACHED, () => {
                console.log(`${janusStreamingPluginHandle} manager handle detached`);
            });
            // Janode exports "EVENT" property with core events
            janusStreamingPluginHandle.on(Janode.EVENT.HANDLE_WEBRTCUP, (_data: any) =>
                console.log("webrtcup event")
            );
            janusStreamingPluginHandle.on(Janode.EVENT.HANDLE_SLOWLINK, (evtdata: any) => {
                console.log("slowlink event", evtdata);
            });
            janusStreamingPluginHandle.on(Janode.EVENT.HANDLE_HANGUP, (evtdata: any) =>
                console.log("hangup event", evtdata)
            );
            janusStreamingPluginHandle.on(Janode.EVENT.HANDLE_DETACHED, (evtdata: any) =>
                console.log("detached event", evtdata)
            );

            janusWsConnection.on(Janode.EVENT.CONNECTION_CLOSED, () => {
                console.log(`Connection with ${janusUri} closed`);
            });


            janusWsConnection.on(
                Janode.EVENT.CONNECTION_ERROR,
                ({ message }: { message: any }) => {
                    console.log(`Connection with Janus error (${message})`);

                    // TODO notify clients via alert

                    // TODO reconnect
                    // notify clients
                }
            );
            janusStreamingPluginHandle.on(
                StreamingPlugin.EVENT.STREAMING_STATUS,
                (evtdata: any) => {
                    console.log(
                        `${janusStreamingPluginHandle.name
                        } streaming handle event status ${JSON.stringify(evtdata)}`
                    );
                }
            );
            if (streamList.length > 0 && this.selectedStream == undefined) {
                console.log("Setting selected stream to:", streamList[0])
                this.$patch({ selectedStream: streamList[0] })
            }
            return true
        },
        async connect(): Promise<void> {
            this.$patch({ status: ConnectionStatus.ConnectionLoading })
            const natsOk = await this.connectNats();
            const janusOk = await this.connectJanus();
            if (natsOk && janusOk) {
                this.$patch({ status: ConnectionStatus.ConnectionReady });
            } else {
                this.$patch({ status: ConnectionStatus.ConnectionError });
            }
        },

        // async publish_command(req: api.PolymorphicPiCommandRequest) {
        //     const natsClient = await this.connect();
        //     const jsonCodec = JSONCodec<api.PolymorphicPiCommandRequest>();
        //     const subject = req.subject_pattern.replace("{pi_id}", req.pi.toString());
        //     await natsClient?.publish(subject, jsonCodec.encode(req));
        //     console.log(`Published to ${subject}`, req);
        // },
        getDetectionAlerts(df: Array<QcDataframeRow>): Array<DetectionAlert> {
            if (df.length < 10) {
                console.warn("Skipping getDetectionAlerts(), less than 10 datapoints available");
                return []
            }
            const nozzleDetected = atLeast(df.map(el => el.nozzle__count > 0), 0.3);
            const printDetected = atLeast(df.map(el => el.print__count > 0), 0.4);
            const raftDetected = atLeast(df.map(el => el.raft__count > 0), 0.3);
            const adhesionFailureDetected = atLeast(df.map(el => el.adhesion__count > 0), 0.15);
            const spaghettiFailureDetected = atLeast(df.map(el => el.spaghetti__count > 0), 0.15);
            if (!nozzleDetected) {
                const alert: DetectionAlert = {
                    id: "nozzle", header: "Calibration: Nozzle", icon: ExclamationTriangleIcon, color: "indigo", description: "Calibration needed to improve nozzle monitoring."
                }
                const showAlert = this.detectionAlerts.filter(a => a.id === alert.id).length === 0;
                if (showAlert) {
                    this.detectionAlerts.push(alert)
                }
            }
            if (!printDetected) {
                const alert: DetectionAlert = {
                    id: "print", header: "Calibration: Printer", icon: ExclamationTriangleIcon, color: "indigo", description: "Calibration needed to improve print object detection."
                }
                const printerAlertShown = this.detectionAlerts.filter(a => a.id === alert.id);
                if (!printerAlertShown) {
                    this.detectionAlerts.push(alert)
                }
            }

            if (!raftDetected) {
                const alert: DetectionAlert = {
                    id: "raft", header: "Calibration: Raft", icon: ExclamationTriangleIcon, color: "indigo", description: "Calibration needed to improve raft detection."
                }

                const showAlert = this.detectionAlerts.filter(a => a.id === alert.id).length === 0;
                if (showAlert) {
                    this.detectionAlerts.push(alert)
                }
            }

            if (adhesionFailureDetected) {
                const alert: DetectionAlert = {
                    id: "adhesion", header: "Failure: Bed Adhesion", icon: ExclamationTriangleIcon, color: "red", description: "Critical failures detected. Pausing 3D print job."
                }
                const showAlert = this.detectionAlerts.filter(a => a.id === alert.id).length === 0;
                if (showAlert) {
                    this.detectionAlerts.push(alert)
                }
            }
            if (spaghettiFailureDetected) {
                const alert: DetectionAlert = {
                    id: "spaghetti", header: "Failure: Spaghetti", icon: ExclamationTriangleIcon, color: "red", description: "Critical failures detected. Pausing 3D print job."
                }
                const showAlert = this.detectionAlerts.filter(a => a.id === alert.id).length === 0;
                if (showAlert) {
                    this.detectionAlerts.push(alert)
                }
            }
            return this.detectionAlerts
        },

        async subscribeQcDataframes() {
            if (this.natsConnection == undefined) {
                return;
            }

            const natsClient = toRaw(this.natsConnection);
            // create a JSON codec/decoder
            const jsonCodec = JSONCodec<Array<QcDataframeRow>>();

            // this subscription listens for all Pi events (scoped to NATs account/org)
            const sub = natsClient.subscribe("pi.qc");
            (async (sub: Subscription) => {
                console.log(`Subscribed to ${sub.getSubject()} events...`);
                for await (const msg of sub) {
                    const df: Array<QcDataframeRow> = jsonCodec.decode(
                        msg.data
                    );
                    this.getDetectionAlerts(df);
                    this.$patch({ df });
                    console.log("Deserialized dataframe", df);
                }
                console.log(`subscription ${sub.getSubject()} drained.`);
            })(sub);
        },

        pushAlert(alert: UiAlert) {
            // show at most 1 alert message with the same header
            const alreadyShown = this.alerts.filter(
                (a) => a.header == alert.header
            );
            if (alreadyShown.length === 0) {
                this.alerts.push(alert);
            }
        },
        async trickle(event: any) {
            const { candidate } = event;
            if (this.janusStreamingPluginHandle === undefined) {
                console.warn("trickle() called with undefined janusStreamingPluginHandle");
                return
            }
            const janusStreamingPluginHandle = toRaw(this.janusStreamingPluginHandle);

            if (candidate === undefined) {
                janusStreamingPluginHandle.trickleComplete().catch((e: any) => {
                    console.error("trickleComplete error", e);
                });
            } else {
                janusStreamingPluginHandle.trickle(candidate).catch((e: any) => {
                    console.error("trickle error", e);
                });
            }
        },
        async stopAllStreams() {
            const videoEl = document.getElementById(
                "janus-video"
            ) as HTMLVideoElement;
            if (videoEl == null) {
                console.warn("Failed to get #janus-video element");
            }
            if (videoEl?.srcObject) {
                console.log("Stopping stream");
                (<MediaStream>videoEl.srcObject)
                    .getTracks()
                    .forEach((stream) => stream.stop());
                videoEl.srcObject = null;
            }
            this.$patch({ status: ConnectionStatus.ConnectionReady })
            // await eventsStore.publish_command(req);
        },

        async closePC() {
            if (this.janusPeerConnection !== undefined) {
                const janusPeerConnection = toRaw(this.janusPeerConnection);

                console.log("stopping PeerConnection");
                janusPeerConnection.close();
                this.$patch({ janusPeerConnection: undefined });
            }
        },
        async jsepAnswer(offer: any) {
            const pc = new RTCPeerConnection({
                iceServers: [
                    {
                        urls: "stun:stun.l.google.com:19302",
                    },
                ],
            });
            pc.onnegotiationneeded = (event) =>
                console.log("pc.onnegotiationneeded", event);
            pc.onicecandidate = (event) =>
                this.trickle({ candidate: event.candidate });
            pc.oniceconnectionstatechange = () => {
                console.log(
                    "pc.oniceconnectionstatechange => " + pc.iceConnectionState
                );
                if (
                    pc.iceConnectionState === "failed" ||
                    pc.iceConnectionState === "closed"
                ) {
                    console.warn("Stopping all streams and closing peer connection");
                    this.stopAllStreams();
                    this.closePC();
                }
            };
            pc.ontrack = (event) => {
                console.log("pc.ontrack", event);

                event.track.onunmute = (evt) => {
                    console.log("track.onunmute", evt);
                    /* TODO set srcObject in this callback */
                };
                event.track.onmute = (evt) => {
                    console.log("track.onmute", evt);
                };
                event.track.onended = (evt) => {
                    console.log("track.onended", evt);
                };

                const remoteStream = event.streams[0];
                this.setVideoElement(remoteStream);
            };

            this.$patch({ janusPeerConnection: pc });
            await pc.setRemoteDescription(offer);
            console.log("set remote sdp OK");
            const answer = await pc.createAnswer();
            console.log("create answer OK");
            pc.setLocalDescription(answer);
            console.log("set local sdp OK");
            return answer;
        },

        async startStream() {
            if (this.selectedStream == undefined) {
                console.warn("startStream() was called, but no stream is selected");
                return
            }
            this.$patch({ status: ConnectionStatus.ConnectionStreamLoading, detectionAlerts: [] });
            const janusStreamingPluginHandle = toRaw(this.janusStreamingPluginHandle);
            const media = toRaw(this.selectedStream.media);
            const watchdata = {
                id: this.selectedStream.id,
                media
            };
            console.log("Sending watchdata", watchdata);
            const { jsep, _restart = false } = await janusStreamingPluginHandle.watch(watchdata);
            console.log(`Received offer`, jsep);

            const answer = await this.jsepAnswer(jsep);
            const { status, id } = await janusStreamingPluginHandle.start({ jsep: answer });
            console.log(`start ${id} response sent with status ${status}`);
        },
        async setVideoElement(mediaStream: any) {
            if (!mediaStream) {
                return;
            }
            const videoEl = document.getElementById(
                "janus-video"
            ) as HTMLVideoElement;
            if (videoEl == null) {
                console.warn("Failed to get #janus-video element");
            }
            videoEl.srcObject = mediaStream;
            console.log("Setting videoEl mediastream", videoEl, mediaStream);
            videoEl.play();
            this.$patch({ status: ConnectionStatus.ConnectionStreamReady });
        },
    },
});

if (import.meta.hot) {
    import.meta.hot.accept(acceptHMRUpdate(useEventStore, import.meta.hot));
}