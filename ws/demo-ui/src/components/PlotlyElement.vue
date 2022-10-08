<template>
    <div :id="plotId"></div>
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
  title: 'PrintNanny Quality Monitor',
  font: { size: 12 },
  legend: {
    x: 0,
    y: -0.5,
  }
};


const config = { responsive: true }

onMounted(() => {
    const nozzlePlot: Plotly.PlotData = {
      x: store.meter_x,
      y: store.meter_y_nozzle,
      mode: 'markers',
      name: 'Ok: Nozzle'
    };

    const printPlot: Plotly.PlotData = {
      x: store.meter_x,
      y: store.meter_y_print,
      mode: 'markers',
      name: 'Ok: Print'
    };

    const raftPlot: Plotly.PlotData = {
      x: store.meter_x,
      y: store.meter_y_raft,
      mode: 'markers',
      name: 'Ok: Raft'
    };

    const adhesionPlot: Plotly.PlotData = {
      x: store.meter_x,
      y: store.meter_y_raft,
      mode: 'markers',
      name: 'Defect: Layer Warping'
    };

    const spaghettiPlot: Plotly.PlotData = {
      x: store.meter_x,
      y: store.meter_y_raft,
      mode: 'markers',
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
      y: store.meter_y_nozzle,
      mode: 'markers',
      name: 'Ok: Nozzle'

  };
    const printPlot: Plotly.PlotData = {
      x: store.meter_x,
      y: store.meter_y_print,
      mode: 'markers',
      name: 'Ok: Print'
    };

    const raftPlot: Plotly.PlotData = {
      x: store.meter_x,
      y: store.meter_y_raft,
      mode: 'markers',
      name: 'Ok: Raft'
    };

    const adhesionPlot: Plotly.PlotData = {
      x: store.meter_x,
      y: store.meter_y_raft,
      mode: 'markers',
      name: 'Defect: Layer Warping',
    };

    const spaghettiPlot: Plotly.PlotData = {
      x: store.meter_x,
      y: store.meter_y_raft,
      mode: 'markers',
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