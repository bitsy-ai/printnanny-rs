import { defineStore, acceptHMRUpdate } from "pinia";
import { connect, JSONCodec, credsAuthenticator } from "nats.ws";
import type { NatsConnection, Subscription } from "nats.ws";

import type { QcDataframeRow } from "@types";
import { ConnectionStatus } from "@/types";

function getNatsURI() {
    const hostname = window.location.hostname;
    const uri = `ws://${hostname}:${import.meta.env.VITE_PRINTNANNY_EDGE_NATS_WS_PORT}`;
    console.log(`Connecting to NATS server: ${uri}`)
    return uri
}

export const useEventStore = defineStore({
    id: "nats",
    state: () => ({
        df: [] as Array<QcDataframeRow>,
        natsClient: undefined as NatsConnection | undefined,
        status: ConnectionStatus.Pending as ConnectionStatus
    }),
    actions: {
        async connect(): Promise<NatsConnection | undefined> {
            // create nats connection if not initialized
            if (this.natsClient === undefined) {
                const servers = [getNatsURI()];
                const connectOptions = {
                    servers,
                    debug: false,
                };

                if (import.meta.env.VITE_PRINTNANNY_DEBUG == true) {
                    connectOptions.debug = true;
                }
                const natsClient = await connect(connectOptions);
                console.log(`Initialized NATs connection to ${servers}`);
                this.$patch({ natsClient });
                return natsClient;
            } else {
                return this.natsClient;
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
        push(event: Array<QcDataframeRow>) {
        },
    },
});

if (import.meta.hot) {
    import.meta.hot.accept(acceptHMRUpdate(useEventStore, import.meta.hot));
}