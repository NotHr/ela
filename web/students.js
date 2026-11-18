"use strict";
(() => {
  let students = [],
    profile = null,
    selectedId = null,
    selectedSeries = null,
    signature = "";
  const dialog = $("#student-dialog");
  function list() {
    const search = $("#student-search").value.trim().toLocaleLowerCase();
    const matches = students.filter((student) =>
      `${student.student_id} ${student.name} ${student.classroom}`
        .toLocaleLowerCase()
        .includes(search),
    );
    const body = $("#student-rows");
    body.replaceChildren();
    if (!matches.length) {
      const row = node("tr"),
        cell = node("td", t("noMatch"), "empty-state");
      cell.colSpan = 6;
      row.append(cell);
      body.append(row);
    }
    for (const student of matches) {
      const row = node("tr"),
        name = document.createElement("td"),
        button = node("button", student.name, "record-link");
      button.type = "button";
      button.addEventListener("click", () => open(student.student_id));
      name.append(button);
      row.append(
        node("td", student.student_id),
        name,
        node("td", student.classroom),
        node("td", number(student.assessments, 0)),
        node("td", number(student.completed, 0)),
        node("td", dateLabel(student.last_assessed_at), "date-cell"),
      );
      body.append(row);
    }
  }
  function graph(series) {
    const ns = "http://www.w3.org/2000/svg",
      svg = document.createElementNS(ns, "svg");
    svg.setAttribute("viewBox", "0 0 640 150");
    svg.classList.add("progress-chart");
    svg.setAttribute("role", "img");
    svg.setAttribute("aria-label", t("readingTrend"));
    for (const value of [0, 50, 100]) {
      const y = 130 - value * 1.1,
        line = document.createElementNS(ns, "line"),
        text = document.createElementNS(ns, "text");
      for (const [key, val] of Object.entries({
        x1: 35,
        x2: 625,
        y1: y,
        y2: y,
      }))
        line.setAttribute(key, val);
      text.setAttribute("x", 0);
      text.setAttribute("y", y + 4);
      text.textContent = String(value);
      svg.append(line, text);
    }
    const times = series.points.map((point) =>
        new Date(point.recorded_at).getTime(),
      ),
      first = Math.min(...times),
      last = Math.max(...times);
    const points = series.points.map((point, index) => {
      const x =
        last > first
          ? 40 + ((times[index] - first) / (last - first)) * 575
          : 40 +
            (series.points.length > 1
              ? index / (series.points.length - 1)
              : 0) *
              575;
      return [x, 130 - point.reading_match * 1.1];
    });
    const line = document.createElementNS(ns, "polyline");
    line.setAttribute("points", points.map((p) => p.join(",")).join(" "));
    svg.append(line);
    for (const [index, [x, y]] of points.entries()) {
      const circle = document.createElementNS(ns, "circle"),
        title = document.createElementNS(ns, "title");
      circle.setAttribute("cx", x);
      circle.setAttribute("cy", y);
      circle.setAttribute("r", 3.5);
      title.textContent = `${dateLabel(series.points[index].recorded_at)} · ${number(series.points[index].reading_match)}%`;
      circle.append(title);
      svg.append(circle);
    }
    return svg;
  }
  function detail(force = false) {
    if (!profile) return;
    const current = JSON.stringify(profile) + ui + selectedSeries;
    if (!force && signature === current) return;
    signature = current;
    $("#student-title").textContent = profile.student.name;
    $("#student-meta").textContent =
      `${profile.student.student_id} · ${profile.student.classroom}`;
    const container = $("#student-content");
    container.replaceChildren();
    const totals = node("dl", "", "metrics");
    metric(totals, "assessmentCount", profile.student.assessments);
    metric(totals, "completedCount", profile.student.completed);
    metric(totals, "processing", profile.student.processing);
    metric(totals, "failed", profile.student.failed);
    container.append(totals);
    if (profile.series.length) {
      const label = node("label", t("comparablePassage")),
        select = document.createElement("select");
      for (const series of profile.series) {
        const option = node(
          "option",
          `${series.passage} · ${series.points.length} · ${series.language}`,
        );
        option.value = series.key;
        select.append(option);
      }
      const chosen =
        profile.series.find((series) => series.key === selectedSeries) ||
        profile.series[0];
      selectedSeries = chosen.key;
      select.value = chosen.key;
      select.addEventListener("change", () => {
        selectedSeries = select.value;
        detail(true);
      });
      label.append(select);
      container.append(label);
      if (!chosen.comparable)
        container.append(badge(t("unknownModel"), "provisional"));
      container.append(graph(chosen));
      const changes = node("dl", "", "metrics");
      metric(changes, "changePp", chosen.reading_match_change_pp);
      metric(changes, "paceChange", chosen.pace_change);
      container.append(changes);
      container.append(
        node("p", chosen.model_revision || t("unknownModel"), "muted"),
      );
    } else container.append(node("p", t("noSeries"), "muted"));
    container.append(node("h3", t("studentHistory")));
    const wrap = node("div", "", "table-scroll"),
      table = document.createElement("table"),
      head = document.createElement("thead"),
      tr = document.createElement("tr"),
      body = document.createElement("tbody");
    for (const key of ["date", "activityName", "status", "readingScore"])
      tr.append(node("th", t(key)));
    head.append(tr);
    table.append(head);
    for (const history of profile.history) {
      const row = document.createElement("tr"),
        cell = document.createElement("td"),
        link = node("button", history.activity, "record-link");
      link.type = "button";
      link.addEventListener("click", () => openReport(history.id));
      cell.append(link);
      const status = document.createElement("td");
      status.append(
        badge(
          t(history.status),
          history.status === "complete"
            ? "complete"
            : history.status === "failed"
              ? "failed"
              : "processing",
        ),
      );
      row.append(
        node("td", dateLabel(history.recorded_at), "date-cell"),
        cell,
        status,
        node(
          "td",
          history.reading_match == null
            ? "—"
            : `${number(history.reading_match)}%`,
        ),
      );
      body.append(row);
    }
    table.append(body);
    wrap.append(table);
    container.append(wrap);
  }
  async function loadProfile(id) {
    const result = await api(`/api/students/${encodeURIComponent(id)}`);
    if (selectedId === id) {
      profile = result;
      detail();
    }
  }
  async function open(id) {
    selectedId = id;
    selectedSeries = null;
    signature = "";
    try {
      await loadProfile(id);
      if (!dialog.open) dialog.showModal();
    } catch (error) {
      notify(error.message, true);
    }
  }
  async function load() {
    try {
      students = await api("/api/students");
      list();
      if (dialog.open && selectedId) await loadProfile(selectedId);
    } catch {
      /* Connection status is handled by the main workspace. */
    }
  }
  $("#student-search").addEventListener("input", list);
  $("#export-student").addEventListener("click", () => {
    if (profile)
      download(
        {
          profile,
          assessments: records.filter(
            (record) => record.student_id === profile.student.student_id,
          ),
        },
        `ela-student-${profile.student.student_id}.json`,
      );
  });
  window.addEventListener("ela:refresh", load);
  window.addEventListener("ela:language", () => {
    $("#student-search").placeholder = t("searchStudents");
    list();
    detail(true);
  });
  load();
})();
