<template>
<div class="flex">
    <!-- spinner -->
    <TextSpinner  v-if="store.status == ConnectionStatus.Pending" text="Connecting to Pi..."/>
</div>
</template>

<script setup lang="ts">
import { ConnectionStatus } from "@/types";
import { useEventStore } from "@/stores/events";
import TextSpinner from "@/components/TextSpinner.vue";

const store = useEventStore();
store.connect();
</script>