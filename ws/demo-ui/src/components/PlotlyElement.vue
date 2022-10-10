<template>
    <div :id="plotId" class="m-auto"></div>
</template>
<script setup lang="ts">
import * as Plotly from 'plotly.js-dist-min';
import type { PropType } from 'vue'
import { v4 as uuidv4 } from 'uuid';
import { useEventStore } from "@/stores/events";
import { onMounted } from '@vue/runtime-core';
const store = useEventStore();


const props = defineProps({
    plotId: {
        type: String,
        default: uuidv4,
    },
});

const plotlyLayout = { 
  title: 'PrintNanny Observations',
  margin: {
    l: 50,
    r: 50,
    b: 50,
    t: 50,
    pad: 2
  },
  font: { size: 12 },
  yaxis: {
  //  autorange: true,
    range: [0.4, 1],
    type: 'linear'
  },
} as Plotly.Layout;


const config = { responsive: true, autosize: true }

onMounted(() => {
    const nozzlePlot = {
      x: store.meter_x,
      y: store.meter_y_nozzle_mean,
      error_y: {
        type: 'data',
        array: store.meter_y_nozzle_std,
        visible: true,
        opacity: 0.3

      },
      mode: 'lines+markers',
      name: 'Ok: Nozzle'
    } as Plotly.PlotData;

    const printPlot = {
      x: store.meter_x,
      y: store.meter_y_print_mean,
      error_y: {
        type: 'data',
        array: store.meter_y_print_std,
        visible: true,
        opacity: 0.3

      },
      mode: 'lines+markers',
      name: 'Ok: Print'
    } as Plotly.PlotData;

    const raftPlot = {
      x: store.meter_x,
      y: store.meter_y_raft_mean,
      error_y: {
        type: 'data',
        array: store.meter_y_raft_std,
        visible: true,
        opacity: 0.3

      },
      mode: 'lines+markers',
      name: 'Ok: Raft'
    } as Plotly.PlotData;

    const adhesionPlot = {
      x: store.meter_x,
      y: store.meter_y_adhesion_mean,
      error_y: {
        type: 'data',
        array: store.meter_y_adhesion_std,
        visible: true,
        opacity: 0.3

      },
      mode: 'lines+markers',
      name: 'Defect: Warping'
    } as Plotly.PlotData;

    const spaghettiPlot = {
      x: store.meter_x,
      y: store.meter_y_spaghetti_mean,
      error_y: {
        type: 'data',
        array: store.meter_y_spaghetti_std,
        visible: true,
        opacity: 0.3

      },
      mode: 'lines+markers',
      name: 'Defect: Spaghetti/Adhesion'
    } as Plotly.PlotData;

    const plots = [
      nozzlePlot,
      printPlot,
      raftPlot,
      adhesionPlot,
      spaghettiPlot
    ];
    if (props.plotId !== undefined){
      Plotly.react(props.plotId, plots, plotlyLayout, config)
    }
})
store.$subscribe(() => {
  const nozzlePlot = {
      x: store.meter_x,
      y: store.meter_y_nozzle_mean,
      error_y: {
        type: 'data',
        array: store.meter_y_nozzle_std,
        visible: true,
        opacity: 0.3
      },
      mode: 'lines+markers',
      name: 'Ok: Nozzle'

  } as Plotly.PlotData;
    const printPlot = {
      x: store.meter_x,
      y: store.meter_y_print_mean,
      error_y: {
        type: 'data',
        array: store.meter_y_print_std,
        visible: true,
        opacity: 0.3
      },
      mode: 'lines+markers',
      name: 'Ok: Print'
    } as Plotly.PlotData;

    const raftPlot = {
      x: store.meter_x,
      y: store.meter_y_raft_mean,
      error_y: {
        type: 'data',
        array: store.meter_y_raft_std,
        visible: true,
                opacity: 0.3
      },
      mode: 'lines+markers',
      name: 'Ok: Raft'
    } as Plotly.PlotData;

    const adhesionPlot = {
      x: store.meter_x,
      y: store.meter_y_adhesion_mean,
      error_y: {
        type: 'data',
        array: store.meter_y_adhesion_std,
        visible: true,
        opacity: 0.3
      },
      mode: 'lines+markers',
      name: 'Defect: Layer Warping',
    } as Plotly.PlotData;

    const spaghettiPlot = {
      x: store.meter_x,
      y: store.meter_y_spaghetti_mean,
      error_y: {
        type: 'data',
        array: store.meter_y_spaghetti_std,
        visible: true,
        opacity: 0.3
      },
      mode: 'lines+markers',
      name: 'Defect: Spaghetti/Adhesion'
    } as Plotly.PlotData;

    const plots = [
      nozzlePlot,
      printPlot,
      raftPlot,
      adhesionPlot,
      spaghettiPlot
    ];
    Plotly.react(props.plotId, plots, plotlyLayout, config)
})
</script>