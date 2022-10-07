<template>
    <div :id="plotId"></div>'
</template>
<script setup lang="ts">
import * as Plotly from 'plotly.js-dist-min';
import { v4 as uuidv4 } from 'uuid';
import { useEventStore } from "@/stores/events";
import { onMounted } from '@vue/runtime-core';


const props = defineProps({
    plotId: {
        type: String,
        default: uuidv4,
    }

});

const data: Plotly.BarData[] = [
  {
    x: ['giraffes', 'orangutans', 'monkeys'],
    y: [20, 14, 23],
    type: 'bar'
  }
];

const layout = { 
  title: 'Responsive to window\'s size!',
  font: { size: 18 },
};

const config = { responsive: true }

onMounted(() => {
    Plotly.newPlot(props.plotId || uuidv4(), data, layout, config)
})


const store = useEventStore();
</script>