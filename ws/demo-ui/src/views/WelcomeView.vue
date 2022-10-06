<template>
    <div class="py-10">
      <header>
        <div class="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8">
          <h1 class="text-3xl font-bold text-gray-900 prose">{{ pageTitle }}</h1>
        </div>
      </header>
      <main>
        <div class="mx-auto max-w-7xl sm:px-6 lg:px-8">
          <!-- Replace with your content -->
          <div class="px-4 py-8 sm:px-0">
            <div class="h-96 rounded-lg border-4 border-dashed border-gray-200">
              <RouterView />
            </div>
          </div>
          <!-- /End replace -->
        </div>
      </main>
    </div>
</template>

<script setup lang="ts">

const pageTitle = "PrintNanny monitors 3D print quality. No internet connection required.";
</script>