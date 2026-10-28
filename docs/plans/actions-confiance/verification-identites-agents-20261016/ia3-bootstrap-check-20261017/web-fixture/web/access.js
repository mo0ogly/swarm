'use strict';
const form = document.querySelector('form');
form.addEventListener('submit', async event => {
  event.preventDefault();
  const button = form.querySelector('button');
  const error = document.getElementById('access-error');
  const label = button.textContent;
  button.disabled = true;
  button.textContent = form.dataset.loading;
  error.hidden = true;
  try {
    const response = await fetch(form.action, {
      method: 'POST', credentials: 'same-origin', headers: {Accept: 'application/json'},
      body: new URLSearchParams(new FormData(form))
    });
    const result = await response.json();
    if (!response.ok) throw new Error(result.error || form.dataset.offline);
    window.location.assign(result.redirect);
  } catch (failure) {
    error.textContent = failure.message === 'Failed to fetch' ? form.dataset.offline : failure.message;
    error.hidden = false;
    form.querySelector('input[type=password]').focus();
  } finally {
    button.disabled = false;
    button.textContent = label;
  }
});
