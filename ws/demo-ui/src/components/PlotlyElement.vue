<template>
    <div :id="plotId" class="m-auto"></div>
</template>
<script setup lang="ts">
import * as Plotly from 'plotly.js-dist-min';
import { PropType } from 'vue'
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

const plotlyLayout:Plotly.Layout = { 
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
};


const config = { responsive: true, autosize: true }

onMounted(() => {
    const nozzlePlot: Plotly.PlotData = {
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
    };

    const printPlot: Plotly.PlotData = {
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
    };

    const raftPlot: Plotly.PlotData = {
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
    };

    const adhesionPlot: Plotly.PlotData = {
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
    };

    const spaghettiPlot: Plotly.PlotData = {
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
    };

    const plots = [
      nozzlePlot,
      printPlot,
      raftPlot,
      adhesionPlot,
      spaghettiPlot
    ];

    Plotly.react(props.plotId, plots, plotlyLayout, config)
})
store.$subscribe(() => {
  const nozzlePlot: Plotly.PlotData = {
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

  };
    const printPlot: Plotly.PlotData = {
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
    };

    const raftPlot: Plotly.PlotData = {
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
    };

    const adhesionPlot: Plotly.PlotData = {
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
    };

    const spaghettiPlot: Plotly.PlotData = {
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
    };

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