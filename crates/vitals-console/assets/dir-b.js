(() => {
  "use strict";
  window.ConsoleDirections = window.ConsoleDirections || {};
  window.ConsoleDirections.b = (model, h) => {
    const row = (item, index) => `<article class="check-row state-${item.state.toLowerCase()}">
      <span class="check-number" aria-hidden="true">${String(index + 1).padStart(2, "0")}</span>
      <div class="check-copy"><h2>${h.esc(item.title)}</h2><p>${h.esc(item.reason)}</p>${item.detail ? `<p class="detail">${h.esc(item.detail)}</p>` : ""}</div>
      <div class="check-result"><strong><span aria-hidden="true">${h.glyph(item.state)}</span> ${h.esc(item.state)}</strong><span data-age-base="${item.age ?? ""}" data-age-at="${Date.now()}">${h.ageText(item.age)}</span><span>${h.esc(item.expectation)}</span></div>
    </article>`;
    return `<header class="page-head preflight-head"><p class="eyebrow">Preflight / ${h.esc(model.eyebrow)}</p><h1>${h.esc(model.title)}</h1><p>${h.esc(model.intro)}</p></header>
      ${model.sections.map(section => `<section class="checklist" aria-labelledby="check-${model.screen}"><header><h2 id="check-${model.screen}">${h.esc(section.title)}</h2><span>${section.cards.length} checks</span></header>${section.cards.map(row).join("")}</section>`).join("")}`;
  };
})();
