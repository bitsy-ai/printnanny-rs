import { defineStore, acceptHMRUpdate } from "pinia";

export const useVideoStore = defineStore({
  id: "video",
  state: () => ({
    selectedVideo: undefined as undefined | {},
    loading: false
  }),
  actions: {
    startVideo(video) {
      console.log("Video selected", video)
      this.$patch({ selectedVideo: video, loading: true });
    }
  },
});

if (import.meta.hot) {
  import.meta.hot.accept(acceptHMRUpdate(useVideoStore, import.meta.hot));
}