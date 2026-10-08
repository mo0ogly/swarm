const money = cents => new Intl.NumberFormat('fr-FR', { style: 'currency', currency: 'EUR' }).format(cents / 100);
const basket = new Map();
let catalog = [];
let tracking = null;
let submitting = false;
const networkMessage = 'Serveur local inaccessible. Vérifiez que le terminal exécutant app.py est ouvert, puis réessayez.';

function escapeText(value) { const node = document.createElement('span'); node.textContent = value; return node.innerHTML; }
async function api(url, options) {
  let response;
  try { response = await fetch(url, options); }
  catch (_) { throw new Error(networkMessage); }
  let data;
  try { data = await response.json(); }
  catch (_) { throw new Error(`Réponse inattendue du serveur (HTTP ${response.status}).`); }
  if (!response.ok) throw new Error(data.error || `La demande a échoué (HTTP ${response.status}).`);
  return data;
}

function renderCatalog() {
  const onlyVegetarian = document.querySelector('#vegetarian-filter').checked;
  const visible = catalog.filter(pizza => !onlyVegetarian || pizza.vegetarian);
  document.querySelector('#catalog').innerHTML = visible.map(pizza => `
    <article class="pizza-card"><div class="pizza-image">
      <img src="/static/images/${pizza.image}" alt="" width="600" height="600" loading="lazy">
      ${pizza.vegetarian ? '<span class="recipe-label">Végétarienne</span>' : ''}</div>
      <div class="pizza-copy"><div><h3>${escapeText(pizza.name)}</h3><p>${escapeText(pizza.ingredients)}</p></div>
      <div class="pizza-action"><strong>${money(pizza.price_cents)}</strong><button type="button" data-add="${pizza.id}" aria-label="Ajouter ${escapeText(pizza.name)} au panier">Ajouter <span aria-hidden="true">+</span></button></div></div>
    </article>`).join('');
}

function renderBasket() {
  document.querySelector('#form-error').hidden = true;
  const rows = [...basket.entries()].map(([id, quantity]) => ({ pizza: catalog.find(p => p.id === id), quantity }));
  document.querySelector('#basket-items').innerHTML = rows.length ? rows.map(({ pizza, quantity }) => `
    <div class="basket-row"><img src="/static/images/${pizza.image}" alt="" width="62" height="62"><div><strong>${escapeText(pizza.name)}</strong><span>${money(pizza.price_cents)} pièce</span></div>
    <label>Quantité <input type="number" min="0" max="10" value="${quantity}" data-quantity="${pizza.id}" aria-label="Quantité pour ${escapeText(pizza.name)}"></label></div>`).join('') : '<p class="empty-basket">Votre panier est vide.<br><a href="#carte">Trouvez votre préférée dans la carte.</a></p>';
  const subtotal = rows.reduce((sum, row) => sum + row.pizza.price_cents * row.quantity, 0);
  const delivery = subtotal ? 290 : 0;
  document.querySelector('#subtotal').textContent = money(subtotal);
  document.querySelector('#delivery').textContent = money(delivery);
  document.querySelector('#total').textContent = money(subtotal + delivery);
  document.querySelector('#basket-count').textContent = rows.reduce((sum, row) => sum + row.quantity, 0);
}

async function submitOrder(event) {
  event.preventDefault();
  const form = event.currentTarget;
  const error = document.querySelector('#form-error');
  error.hidden = true;
  if (submitting || !form.reportValidity()) return;
  const button = form.querySelector('button[type="submit"]');
  submitting = true; button.disabled = true; button.textContent = 'Envoi de la commande…';
  const payload = {
    customer_name: form.customer_name.value,
    address: form.address.value,
    phone: form.phone.value,
    items: [...basket.entries()].map(([pizza_id, quantity]) => ({ pizza_id, quantity }))
  };
  try {
    const data = await api('/api/orders', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify(payload) });
    tracking = { number: data.number, token: data.tracking_token };
    sessionStorage.setItem('casa-tracking', JSON.stringify(tracking));
    document.querySelector('#confirmation').hidden = false;
    document.querySelector('#tracking-link').hidden = false;
    document.querySelector('#confirmation-text').textContent = `Commande ${data.number} · total ${money(data.total_cents)}.`;
    document.querySelector('#tracking-error').hidden = true;
    renderTracking(data.status);
    basket.clear(); renderBasket();
    document.querySelector('#basket-feedback').textContent = '';
    document.querySelector('#confirmation').scrollIntoView({ behavior: 'smooth' });
  } catch (reason) { error.textContent = reason.message; error.hidden = false; }
  finally { submitting = false; button.disabled = false; button.innerHTML = 'Confirmer la commande <span aria-hidden="true">→</span>'; }
}

const statusLabels = { recue: 'Reçue', preparation: 'En préparation', livraison: 'En livraison', livree: 'Livrée' };
function renderTracking(status) {
  const keys = Object.keys(statusLabels); const current = keys.indexOf(status);
  document.querySelector('#tracking-steps').innerHTML = keys.map((key, index) => `<div class="step ${index <= current ? 'active' : ''}" aria-current="${key === status ? 'step' : 'false'}"><span>${index < current ? '✓' : index + 1}</span>${statusLabels[key]}</div>`).join('');
  document.querySelector('#tracking-message').textContent = status === 'livree' ? 'Commande livrée dans la démonstration. Bon appétit !' : 'Le restaurant fait avancer le statut de votre commande.';
}
async function refreshTracking() {
  if (!tracking) return;
  const error = document.querySelector('#tracking-error'); error.hidden = true;
  try {
    const data = await api(`/api/orders/${encodeURIComponent(tracking.number)}?token=${encodeURIComponent(tracking.token)}`);
    document.querySelector('#confirmation-text').textContent = `Commande ${data.number} · total ${money(data.total_cents)}.`;
    renderTracking(data.status);
  } catch (reason) { error.textContent = reason.message; error.hidden = false; }
}

document.querySelector('#catalog').addEventListener('click', event => {
  const button = event.target.closest('[data-add]'); if (!button) return;
  const id = button.dataset.add; const quantity = basket.get(id) || 0;
  const feedback = document.querySelector('#basket-feedback');
  if (quantity >= 10) { feedback.textContent = 'Maximum de 10 pizzas par recette dans cet atelier.'; return; }
  basket.set(id, quantity + 1); renderBasket();
  feedback.textContent = `${catalog.find(p => p.id === id).name} ajoutée au panier. Continuez votre sélection ou ouvrez Mon panier.`;
});
document.querySelector('#basket-items').addEventListener('change', event => { const id = event.target.dataset.quantity; if (!id) return; const quantity = Math.max(0, Math.min(10, Number.parseInt(event.target.value, 10) || 0)); if (quantity) basket.set(id, quantity); else basket.delete(id); renderBasket(); });
document.querySelector('#vegetarian-filter').addEventListener('change', renderCatalog);
document.querySelector('#checkout').addEventListener('submit', submitOrder);
document.querySelector('#refresh-status').addEventListener('click', refreshTracking);

api('/api/catalog').then(data => { catalog = data; renderCatalog(); renderBasket(); }).catch(reason => { document.querySelector('#catalog').textContent = reason.message; });
try { tracking = JSON.parse(sessionStorage.getItem('casa-tracking')); } catch (_) { tracking = null; }
if (tracking) { document.querySelector('#confirmation').hidden = false; document.querySelector('#tracking-link').hidden = false; refreshTracking(); }
