(() => {
  "use strict";
  window.ConsoleDirections = window.ConsoleDirections || {};
  window.ConsoleDirections.c = (model, h) => {
    const node = (item, index) => `<article class="flow-node state-${item.state.toLowerCase()}">
      <div class="node-top"><span class="node-index">${index + 1}</span><strong><span aria-hidden="true">${h.glyph(item.state)}</span> ${h.esc(item.state)}</strong></div>
      <h2>${h.esc(item.title)}</h2><p>${h.esc(item.reason)}</p>
      <div class="flow-time"><span data-age-base="${item.age ?? ""}" data-age-at="${Date.now()}">${h.ageText(item.age)}</span><span>${h.esc(item.expectation)}</span></div>
      ${item.detail ? `<p class="detail">${h.esc(item.detail)}</p>` : ""}
    </article>`;
    return `<header class="page-head flow-head"><p class="eyebrow">Flowpath · ${h.esc(model.eyebrow)}</p><h1>${h.esc(model.title)}</h1><p>${h.esc(model.intro)}</p></header>
      ${model.sections.map(section => `<section class="flow-section" aria-labelledby="flow-${model.screen}"><h2 id="flow-${model.screen}">${h.esc(section.title)}</h2><div class="flow-track">${section.cards.map(node).join('<span class="flow-arrow" aria-hidden="true">→</span>')}</div></section>`).join("")}`;
  };
})();
