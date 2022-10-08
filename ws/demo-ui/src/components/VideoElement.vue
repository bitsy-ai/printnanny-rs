<script setup lang="ts">
import { useEventStore } from "@/stores/events";
import { ConnectionStatus } from "@/types";
import { onMounted } from "vue";
import {
    Listbox,
    ListboxButton,
    ListboxOptions,
    ListboxOption,
    ListboxLabel
  } from '@headlessui/vue'
  import {  ChevronUpDownIcon, CheckIcon } from '@heroicons/vue/24/outline'
  import TextSpinner from "@/components/TextSpinner.vue"
import VideoElement from "@/components/VideoElement.vue";
import PlotlyElement from "@/components/PlotlyElement.vue";
const store = useEventStore();
</script>
    
<template>
        <div class="overflow-hidden rounded-lg bg-white px-4 py-5 shadow sm:p-6 col-span-2">
            <h3 class="text-lg font-medium leading-6 text-gray-900 text-center">Video Stream</h3>

            <video
                id="janus-video"
                muted
                class="aspect-video h-80 mx-auto my-4 border-1 border-dashed border-gray-200 bg-gray-200"
                aria-placeholder=" Video stream is loading"
                poster="@/assets/video-paused.svg"
            ></video>
            <div class="grid grid-cols-2 gap-4">
                <Listbox as="div" v-model="store.selectedStream">
                <ListboxLabel class="block text-sm font-medium text-gray-700 text-center">Select a video stream:</ListboxLabel>
                <div class="relative mt-1">
                    <ListboxButton class="relative w-full cursor-default rounded-md border border-gray-300 bg-white py-2 pl-3 pr-10 text-left shadow-sm focus:border-indigo-500 focus:outline-none focus:ring-1 focus:ring-indigo-500 sm:text-sm">
                        <TextSpinner text="Loading stream list" v-if="!store.selectedStream"></TextSpinner>
                        <span class="block truncate" v-else>{{ store.selectedStream.description }}</span>

                        <span class="pointer-events-none absolute inset-y-0 right-0 flex items-center pr-2">
                        <ChevronUpDownIcon class="h-5 w-5 text-gray-400" aria-hidden="true" />
                        </span>
                    </ListboxButton>

                    <transition leave-active-class="transition ease-in duration-100" leave-from-class="opacity-100" leave-to-class="opacity-0">
                        <ListboxOptions class="absolute z-10 mt-1 max-h-60 w-full overflow-auto rounded-md bg-white py-1 text-base shadow-lg ring-1 ring-black ring-opacity-5 focus:outline-none sm:text-sm">
                        <ListboxOption as="template" v-for="stream in store.streamList" :key="stream.id" :value="stream" v-slot="{ active, selected }">
                            <li :class="[active ? 'text-white bg-indigo-600' : 'text-gray-900', 'relative cursor-default select-none py-2 pl-3 pr-9']">
                            <span :class="[selected ? 'font-semibold' : 'font-normal', 'block truncate']">{{ stream.description }}</span>

                            <span v-if="selected" :class="[active ? 'text-white' : 'text-indigo-600', 'absolute inset-y-0 right-0 flex items-center pr-4']">
                                <CheckIcon class="h-5 w-5" aria-hidden="true" />
                            </span>
                            </li>
                        </ListboxOption>
                        </ListboxOptions>
                    </transition>

                </div>
                
                </Listbox>

            <div class="flex">

            <button
            @click="store.startStream()"
            type="button"
            :disabled="store.status !== ConnectionStatus.ConnectionReady"
            :class="[store.status == ConnectionStatus.ConnectionReady ? 'hover:bg-blue-700 hover:shadow-lg focus:bg-blue-700 focus:shadow-lg focus:outline-none focus:ring-0 active:bg-blue-800 active:shadow-lg ': 'focus:ring-0 transition duration-150 ease-in-out pointer-events-none opacity-60', 'block truncate']"

            class="inline-block h-10 px-6 py-2.5 mt-6 bg-blue-600 text-white font-medium text-xs leading-tight uppercase rounded shadow-md hover:bg-blue-700 hover:shadow-lg focus:bg-blue-700 focus:shadow-lg focus:outline-none focus:ring-0 active:bg-blue-800 active:shadow-lg transition duration-150 ease-in-out">
            Start
            </button>
            <button
            @click="store.stopAllStreams()"
            type="button" 
            :disabled="store.status !== ConnectionStatus.ConnectionStreamReady"
            :class="[store.status == ConnectionStatus.ConnectionStreamReady ? 'hover:bg-blue-700 hover:shadow-lg focus:bg-blue-700 focus:shadow-lg focus:outline-none focus:ring-0 active:bg-blue-800 active:shadow-lg ': 'focus:ring-0 transition duration-150 ease-in-out pointer-events-none opacity-60', 'block truncate']"
            class="inline-block h-10 px-6 py-2.5 mt-6 ml-4 bg-blue-600 text-white font-medium text-xs leading-tight uppercase rounded shadow-md hover:bg-blue-700 transition duration-150 ease-in-out">
            Stop
            </button>
            </div>
            </div>
        </div>
</template>
