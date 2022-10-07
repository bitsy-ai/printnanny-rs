import { useEventStore } from "@/stores/events";
import type { UiAlert, AlertAction } from "@/types";

function handleError(header: string, e: Error) {
  let message = e.message || e.toString();
  const actions = [
    {
      color: "red",
      text: "Refresh",
      onClick: () => {
        window.location.reload();
      },
    },
  ] as Array<AlertAction>;
  const alert: UiAlert = {
    header,
    message,
    actions,
    error: e,
  };
  const store = useEventStore();
  store.pushAlert(alert);
}

function alertMessage(header: string, message: string) {
  const actions = [
    {
      color: "amber",
      text: "Refresh Page",
      onClick: () => {
        window.location.reload();
      },
    },
  ] as Array<AlertAction>;
  const alert: UiAlert = {
    header,
    message,
    actions,
    error: undefined
  };
  const store = useEventStore();
  store.pushAlert(alert);
}

export { handleError }