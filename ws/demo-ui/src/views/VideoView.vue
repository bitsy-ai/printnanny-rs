<template>
<main class="py-5">
    <div class="mx-auto max-w-7xl sm:px-6 lg:px-8 justify-center w-full grid grid-cols-4 gap-4 content-center pb-5">


    </div>
    <div class="mx-auto max-w-7xl sm:px-6 lg:px-8 border-4 border-dashed border-gray-200">
        <!-- Replace with your content -->
        <div class="px-4 py-8 sm:px-0 grid grid-cols-3 gap-2 flex">

            <VideoElement class="col-span-2 flex-1"/>
            <DetectionSummary class="col-span-1" />
            <div class="col-span-3 mt-4">
                <PlotlyElement/>
            </div>
        </div>
        <!-- /End replace -->   
    </div>
</main>
</template>

<script setup lang="ts">
import * as Plotly from 'plotly.js-dist-min';
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
import { ConnectionStatus } from "@/types";
import TextSpinner from "@/components/TextSpinner.vue"
import VideoElement from "@/components/VideoElement.vue";
import PlotlyElement from "@/components/PlotlyElement.vue";
import DetectionSummary from "@/components/DetectionSummary.vue";
const store = useEventStore();


</script>