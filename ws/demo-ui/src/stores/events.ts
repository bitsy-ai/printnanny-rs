import { defineStore, acceptHMRUpdate } from "pinia";
import { connect, JSONCodec, credsAuthenticator } from "nats.ws";
import type { NatsConnection, Subscription } from "nats.ws";
import Janode from "janode";
import StreamingPlugin from "janode/plugins/streaming";

import type { QcDataframeRow } from "@types";
import { ConnectionStatus } from "@/types";

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
        janusWsConnection: undefined as undefined | any,
        janusSession: undefined as undefined | any,
        janusPeerConnection: undefined as undefined | RTCPeerConnection,
        janusStreamingPluginHandle: undefined as undefined | any,
        status: ConnectionStatus.Pending as ConnectionStatus
    }),
    actions: {
        async getStreamList(): Promise<undefined> {
            if (this.janusStreamingPluginHandle === undefined) {
                console.warn("getStreamList callled before Janus streaming handle connected, attempting to connect");
                await this.connect();
                return this.getStreamList()
                return;
            } else {
                const streamList = await this.janusStreamingPluginHandle.list()
                console.log("Got streaming list", streamList);
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
                const natsConnection = await connect(connectOptions);
                console.log(`Initialized NATs connection to ${servers}`);
                this.$patch({ natsConnection });
                return true
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
            const janusWsConnection = await Janode.connect(connectOpts);
            this.$patch({ janusWsConnection: janusWsConnection });
            const janusSession = await janusWsConnection.create();
            this.$patch({ janusSession });
            const janusStreamingPluginHandle = await janusSession.attach(StreamingPlugin);
            this.$patch({ janusStreamingPluginHandle });
            return true
        },
        async connect(): Promise<[boolean, boolean] | undefined> {
            const result = await Promise.all([
                this.connectNats(),
                this.connectJanus()
            ]);
            if (result.every(el => el === true)) {
                this.$patch({ status: ConnectionStatus.Connected })
            }
            return result
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
        push(event: Array<QcDataframeRow>) {
        },
    },
});

if (import.meta.hot) {
    import.meta.hot.accept(acceptHMRUpdate(useEventStore, import.meta.hot));
}