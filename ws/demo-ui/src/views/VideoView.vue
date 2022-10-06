<template>
<div class="py-10">
    <header>
    <div class="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8 flex grid grid-cols-3 justify-center">
        
        <div v-for="video in videos" class="flex h-24 justify-center">
            <a 
                class="hover:bg-indigo-100 flex border-4 border-indigo-500/25 hover:border-indigo-500/50" 
                :class="[selectedVideo && video.name == selectedVideo.name ? 'border-indigo-500 text-gray-900' : '' ]"
                @click="startVideo(video)">
            <img :src="video.src" />
            </a>
        </div>
    </div>
    </header>
    <main>
    <div class="mx-auto max-w-7xl sm:px-6 lg:px-8">
        <!-- Replace with your content -->
        <div class="px-4 py-8 sm:px-0">
        <div class="h-96 rounded-lg border-4 border-dashed border-gray-200">
        </div>
        </div>
        <!-- /End replace -->
    </div>
    </main>
</div>
</template>

<script setup lang="ts">
import { ref } from "vue";
import { useVideoStore } from "@/stores/video"

const selectedVideo = ref();

function startVideo(video){
    console.log("Video selected", video)
    selectedVideo.value = video.name;
}
const videos = [
    { name: "fixture_0.mp4", src:"https://via.placeholder.com/300x200?text=Video1" },
    { name: "fixture_0.mp4", src:"https://via.placeholder.com/300x200?text=Video2" },
    { name: "fixture_0.mp4", src:"https://via.placeholder.com/300x200?text=Video3" }

]
</script>