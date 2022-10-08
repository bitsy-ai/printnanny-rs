<template>
<div class="overflow-hidden rounded-lg bg-white px-4 py-5 shadow sm:p-6 col-span-2">
    <div>
        <h3 class="text-lg font-medium leading-6 text-gray-900">Alerts & Warnings</h3>

        <dl class="mt-5 grid grid-cols-1 gap-5">
        <div v-for="item in store.detectionStats" :key="item.id" class="relative overflow-hidden rounded-lg bg-gray-100 p-4 shadow">
            <dt>
            <div class="absolute rounded-md bg-indigo-500 p-3" v-if="item.color == 'indigo'">
                <component :is="item.icon" class="h-6 w-6 text-white" aria-hidden="true" />
            </div>
            <div class="absolute rounded-md bg-red-500 p-3" v-if="item.color == 'red'">
                <component :is="item.icon" class="h-6 w-6 text-white" aria-hidden="true" />
            </div>
            <p class="ml-16 truncate text-sm font-semibold text-center mb-2">{{ item.id }}</p>
            </dt>
            <dd class="ml-16 flex items-baseline text-center">
            <p class="ml-2 flex items-baseline text-sm text-gray-500">
                {{ item.description }}
            </p>
            </dd>
        </div>
        </dl>
    </div>
</div>
</template>
<script setup lang="ts">
import { computed } from "vue";
import { ArrowDownIcon, ArrowUpIcon, CheckIcon, ExclamationTriangleIcon } from '@heroicons/vue/20/solid'
import { useEventStore } from "@/stores/events";
import { CursorArrowRaysIcon, EnvelopeOpenIcon, UsersIcon } from '@heroicons/vue/24/outline'

const store = useEventStore();

// const stats = computed(() => {
//     return [
//         { id: "nozzle", detected: store.nozzle_detected, icon: store.nozzle_detected ? CheckIcon :  ExclamationTriangleIcon, color: store.nozzle_detected ? "emerald" : "amber" },
//         { id: "nozzle", detected: store.nozzle_detected, icon: store.nozzle_detected ? CheckIcon :  ExclamationTriangleIcon, color: store.nozzle_detected ? "emerald" : "amber" },
//         { id: "nozzle", detected: store.nozzle_detected, icon: store.nozzle_detected ? CheckIcon :  ExclamationTriangleIcon, color: store.nozzle_detected ? "emerald" : "amber" },

//     ]
// })
// const stats = [
//   { id: 1, name: 'Total Subscribers', stat: '71,897', icon: UsersIcon, change: '122', changeType: 'increase' },
//   { id: 2, name: 'Avg. Open Rate', stat: '58.16%', icon: EnvelopeOpenIcon, change: '5.4%', changeType: 'increase' },
//   { id: 3, name: 'Avg. Click Rate', stat: '24.57%', icon: CursorArrowRaysIcon, change: '3.2%', changeType: 'decrease' },
// ]
</script>