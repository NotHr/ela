"use strict";
(() => {
  $("#import-collection").addEventListener("click", () =>
    $("#collection-dialog").showModal(),
  );
  $("#collection-form").addEventListener("submit", async (event) => {
    event.preventDefault();
    const results = $("#collection-results");
    results.replaceChildren();
    $("#collection-submit").disabled = true;
    try {
      const items = await api("/api/collection/import", {
        method: "POST",
        body: new FormData(event.target),
      });
      for (const item of items) {
        const row = node("div", "", "import-result");
        const label = item.id
          ? node("button", item.filename, "record-link")
          : node("span", item.filename);
        if (item.id) {
          label.type = "button";
          label.addEventListener("click", () => openReport(item.id));
        }
        row.append(
          label,
          badge(
            t(item.status),
            item.status === "accepted"
              ? "processing"
              : item.status === "duplicate"
                ? "complete"
                : "failed",
          ),
        );
        if (item.error) row.append(node("span", item.error, "danger"));
        results.append(row);
      }
      await refresh();
    } catch (error) {
      results.append(node("p", error.message, "danger"));
    } finally {
      $("#collection-submit").disabled = false;
    }
  });
})();
