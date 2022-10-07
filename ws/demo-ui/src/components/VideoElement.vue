<script setup lang="ts">
import { useEventStore } from "@/stores/events";
import { ConnectionStatus } from "@/types";
import { onMounted } from "vue";

const store = useEventStore();
</script>
    
<template>
    <div>
    <video
        id="janus-video"
        muted
        class="aspect-video h-80 mx-auto my-4 border-4 border-dashed border-gray-200"
        aria-placeholder=" Video stream is loading"
        poster="@/assets/video-paused.svg"
    ></video>
    <p v-if="store.status == ConnectionStatus.ConnectionReady" class="text-sm text-gray-500 text-center">
        Select a video stream and click the "Start" button to begin.
    </p>
    <p v-if="store.status == ConnectionStatus.ConnectionStreamLoading" class="text-sm text-gray-500 text-center">
        Video stream is loading
    </p>
    </div>
</template>
