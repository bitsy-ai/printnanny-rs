import { defineStore, acceptHMRUpdate } from "pinia";
import { connect, JSONCodec, credsAuthenticator } from "nats.ws";
import type { NatsConnection, Subscription } from "nats.ws";
import Janode from "janode";
import StreamingPlugin from "janode/plugins/streaming";

import type { QcDataframeRow, UiAlert } from "@types";
import { ConnectionStatus } from "@/types";
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
        status: ConnectionStatus.Pending as ConnectionStatus,
        alerts: [] as Array<UiAlert>,
        streamList: [] as Array<any>,

    }),
    actions: {
        async getStreamList(): Promise<undefined> {
            if (this.janusStreamingPluginHandle === undefined) {
                console.warn("getStreamList callled before Janus streaming handle connected");
                return;
            } else {
                const streamList = await this.janusStreamingPluginHandle.list()
                console.log("Got streaming list", streamList);
                return streamList
            }
        },

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
            const streamList = await janusStreamingPluginHandle.list();
            console.log("Found streamlist", streamList);
            this.$patch({ janusStreamingPluginHandle, janusSession, janusWsConnection, streamList });
            return true
        },
        async connect(): Promise<void> {
            const natsOk = await this.connectNats();
            const janusOk = await this.connectJanus();
            if (natsOk && janusOk) {
                this.$patch({ status: ConnectionStatus.Connected });
            }
        },

        // async publish_command(req: api.PolymorphicPiCommandRequest) {
        //     const natsClient = await this.connect();
        //     const jsonCodec = JSONCodec<api.PolymorphicPiCommandRequest>();
        //     const subject = req.subject_pattern.replace("{pi_id}", req.pi.toString());
        //     await natsClient?.publish(subject, jsonCodec.encode(req));
        //     console.log(`Published to ${subject}`, req);
        // },
        async subscribeQcDataframes() {
            const natsClient = await this.connect();
            if (natsClient == undefined) {
                return;
            }
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
                    this.handle(df);
                    this.$patch({ df });
                    console.log("Deserialized dataframe", df);
                }
                console.log(`subscription ${sub.getSubject()} drained.`);
            })(sub);
        },
        handle(event: Array<QcDataframeRow>) {
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
    },
});

if (import.meta.hot) {
    import.meta.hot.accept(acceptHMRUpdate(useEventStore, import.meta.hot));
}