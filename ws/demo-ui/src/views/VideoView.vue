<template>
<div class="py-10">
    <header>
    <div class="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8 flex grid grid-cols-2 justify-center">
        
        <div class="flex flex-1 h-24 justify-center w-full">
            <Listbox as="div" v-model="store.selectedStream">
                <ListboxLabel class="block text-sm font-medium text-gray-700">Select a video stream:</ListboxLabel>
                <div class="relative mt-1">
                <ListboxButton class="relative w-48 cursor-default rounded-md border border-gray-300 bg-white py-2 pl-3 pr-10 text-left shadow-sm focus:border-indigo-500 focus:outline-none focus:ring-1 focus:ring-indigo-500 sm:text-sm">
                    <TextSpinner text="Loading stream list" v-if="store.selectedStream === undefined"></TextSpinner>
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
            <button
                @click="store.startStream()"
                type="button"
                :disabled="store.status !== ConnectionStatus.ConnectionReady"
                :class="[store.status == ConnectionStatus.ConnectionReady ? 'hover:bg-blue-700 hover:shadow-lg focus:bg-blue-700 focus:shadow-lg focus:outline-none focus:ring-0 active:bg-blue-800 active:shadow-lg ': 'focus:ring-0 transition duration-150 ease-in-out pointer-events-none opacity-60', 'block truncate']"

                class="inline-block h-10 px-6 py-2.5 mt-6 ml-4 bg-blue-600 text-white font-medium text-xs leading-tight uppercase rounded shadow-md hover:bg-blue-700 hover:shadow-lg focus:bg-blue-700 focus:shadow-lg focus:outline-none focus:ring-0 active:bg-blue-800 active:shadow-lg transition duration-150 ease-in-out">
                Start
            </button>
            <button
                @click="store.stopAllStreams()"
                type="button" 
                :disabled="store.status !== ConnectionStatus.ConnectionStreaming"
                :class="[store.status == ConnectionStatus.ConnectionStreaming ? 'hover:bg-blue-700 hover:shadow-lg focus:bg-blue-700 focus:shadow-lg focus:outline-none focus:ring-0 active:bg-blue-800 active:shadow-lg ': 'focus:ring-0 transition duration-150 ease-in-out pointer-events-none opacity-60', 'block truncate']"
                class="inline-block h-10 px-6 py-2.5 mt-6 ml-4 bg-blue-600 text-white font-medium text-xs leading-tight uppercase rounded shadow-md hover:bg-blue-700 transition duration-150 ease-in-out">
                Stop
            </button>
        </div>
        <div class="flex flex-1 justify-center space-x-2">

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
import {
    Listbox,
    ListboxButton,
    ListboxOptions,
    ListboxOption,
    ListboxLabel
  } from '@headlessui/vue'
import { useEventStore } from "@/stores/events";
import {  ChevronUpDownIcon, CheckIcon } from '@heroicons/vue/24/outline'
import TextSpinner from "@/components/TextSpinner.vue"
import { ConnectionStatus } from "@/types";

const store = useEventStore();

</script>