(() => {
  "use strict";
  window.ConsoleDirections = window.ConsoleDirections || {};
  window.ConsoleDirections.a = (model, h) => {
    const card = item => `<article class="status-card state-${item.state.toLowerCase()}">
      <div class="status-line"><span class="glyph" aria-hidden="true">${h.glyph(item.state)}</span><strong>${h.esc(item.state)}</strong></div>
      <h2>${h.esc(item.title)}</h2>
      <p class="timing"><span data-age-base="${item.age ?? ""}" data-age-at="${Date.now()}">${h.ageText(item.age)}</span> · ${h.esc(item.expectation)}</p>
      <p>${h.esc(item.reason)}</p>${item.detail ? `<p class="detail">${h.esc(item.detail)}</p>` : ""}
    </article>`;
    return `<header class="page-head"><p class="eyebrow">${h.esc(model.eyebrow)} · Service Mode</p><h1>${h.esc(model.title)}</h1><p>${h.esc(model.intro)}</p></header>
      ${model.sections.map(section => `<section class="service-section" aria-labelledby="section-${model.screen}"><h2 id="section-${model.screen}">${h.esc(section.title)}</h2><div class="card-grid">${section.cards.map(card).join("")}</div></section>`).join("")}`;
  };
})();
