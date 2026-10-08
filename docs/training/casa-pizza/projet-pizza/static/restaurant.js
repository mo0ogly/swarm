const labels = { recue: 'Reçue', preparation: 'En préparation', livraison: 'En livraison', livree: 'Livrée' };
const nextStatus = { recue: 'preparation', preparation: 'livraison', livraison: 'livree' };
const euro = cents => new Intl.NumberFormat('fr-FR', { style: 'currency', currency: 'EUR' }).format(cents / 100);
const safe = value => { const node = document.createElement('span'); node.textContent = value; return node.innerHTML; };
const networkMessage = 'Serveur local inaccessible. Vérifiez que le terminal exécutant app.py est ouvert, puis réessayez.';
async function request(url, options) {
  let response;
  try { response = await fetch(url, options); }
  catch (_) { throw new Error(networkMessage); }
  let data;
  try { data = await response.json(); }
  catch (_) { throw new Error(`Réponse inattendue du serveur (HTTP ${response.status}).`); }
  return {response, data};
}
let currentOrders = [];
function renderOrders() {
  const filter = document.querySelector('#order-filter').value.trim().toUpperCase();
  const orders = currentOrders.filter(order => order.number.includes(filter));
  document.querySelector('#orders-message').textContent = orders.length ? `${orders.length} commande(s)` : 'Aucune commande correspondante.';
    document.querySelector('#orders-list').innerHTML = orders.map(order => `<article class="restaurant-order"><div><p class="eyebrow">${safe(order.number)}</p><h2>${safe(order.customer_name)}</h2><p>${safe(order.address)}</p><ul>${order.items.map(item => `<li>${item.quantity} × ${safe(item.name)}</li>`).join('')}</ul><strong>${euro(order.total_cents)}</strong></div><div><span class="status">${labels[order.status]}</span>${nextStatus[order.status] ? `<button class="button" type="button" data-order="${safe(order.number)}" data-status="${nextStatus[order.status]}">Passer à « ${labels[nextStatus[order.status]]} »</button>` : '<p>Commande terminée</p>'}</div></article>`).join('');
}
async function loadOrders() {
  try {
    const {response, data:orders} = await request('/api/restaurant/orders');
    if (response.status === 401) { document.querySelector('#login-panel').hidden = false; document.querySelector('#orders-panel').hidden = true; return; }
    if (!response.ok) throw new Error(orders.error || 'Chargement impossible.');
    document.querySelector('#login-panel').hidden = true; document.querySelector('#orders-panel').hidden = false;
    currentOrders = orders; renderOrders();
  } catch (reason) {
    const target = document.querySelector('#orders-panel').hidden ? document.querySelector('#login-error') : document.querySelector('#orders-message');
    target.textContent = reason.message; target.hidden = false;
  }
}
document.querySelector('#login-form').addEventListener('submit', async event => {
  event.preventDefault(); const error = document.querySelector('#login-error'); error.hidden = true;
  const button = event.currentTarget.querySelector('button'); button.disabled = true;
  try {
    const {response, data} = await request('/api/restaurant/login', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ password: document.querySelector('#password').value }) });
    if (!response.ok) throw new Error(data.error || 'Connexion impossible.');
    document.querySelector('#password').value = ''; await loadOrders();
  } catch (reason) { error.textContent = reason.message; error.hidden = false; }
  finally { button.disabled = false; }
});
document.querySelector('#orders-list').addEventListener('click', async event => {
  const button = event.target.closest('[data-order]'); if (!button) return;
  button.disabled = true;
  try {
    const {response, data} = await request(`/api/restaurant/orders/${encodeURIComponent(button.dataset.order)}/status`, { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ status: button.dataset.status }) });
    if (!response.ok) throw new Error(data.error || 'Changement impossible.');
    await loadOrders();
  } catch (reason) { document.querySelector('#orders-message').textContent = reason.message; }
  finally { button.disabled = false; }
});
document.querySelector('#refresh-orders').addEventListener('click', loadOrders);
loadOrders();

document.querySelector('#order-filter').addEventListener('input', renderOrders);
