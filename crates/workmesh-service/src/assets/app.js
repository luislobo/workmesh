(() => {
  const stateEl = document.getElementById("ws-state");
  if (!stateEl) return;

  const refreshMs = Number(document.body.dataset.refreshMs || "3000");
  let pollTimer = null;
  let ws = null;

  function setState(text) {
    stateEl.textContent = text;
  }

  function startPolling() {
    if (pollTimer) return;
    setState(`Realtime: polling every ${refreshMs}ms`);
    pollTimer = setInterval(() => {
      window.location.reload();
    }, refreshMs);
  }

  function stopPolling() {
    if (!pollTimer) return;
    clearInterval(pollTimer);
    pollTimer = null;
  }

  function connectWs() {
    const proto = window.location.protocol === "https:" ? "wss" : "ws";
    const url = `${proto}://${window.location.host}/ws`;

    try {
      ws = new WebSocket(url);
    } catch (_err) {
      startPolling();
      return;
    }

    ws.onopen = () => {
      stopPolling();
      setState("Realtime: websocket connected");
    };

    ws.onmessage = (event) => {
      try {
        const payload = JSON.parse(event.data);
        if (payload && (payload.type === "delta" || payload.type === "snapshot")) {
          window.location.reload();
        }
      } catch (_err) {
        // Ignore malformed payloads.
      }
    };

    ws.onerror = () => {
      setState("Realtime: websocket error, falling back to polling");
      startPolling();
    };

    ws.onclose = () => {
      setState("Realtime: websocket disconnected, reconnecting...");
      startPolling();
      setTimeout(connectWs, 1200);
    };
  }

  connectWs();
})();
