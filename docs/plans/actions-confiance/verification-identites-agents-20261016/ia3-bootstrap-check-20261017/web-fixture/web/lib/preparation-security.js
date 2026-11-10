/**
 * Module de sécurité frontend pour l'application WATTSON
 * Protection contre XSS et validation des entrées
 *
 * @author Fabrice Pizzi <fabrice.pizzi@ssi.gouv.fr>
 * @copyright ANSSI
 * @version 2.0 (Sécurisée)
 * @date 2025-10-16
 *
 * Modifications:
 * - 2025-10-16 : Correction VULN-SEC-001 (Rate limiting côté client)
 */

// ============================================================================
// PROTECTION XSS - Sanitization des entrées
// ============================================================================

/**
 * Échappe les caractères HTML pour prévenir XSS
 * @param {string} text - Texte à échapper
 * @returns {string} Texte échappé
 */
function escapeHtml(text) {
    if (text === null || text === undefined) {
        return '';
    }

    const map = {
        '&': '&amp;',
        '<': '&lt;',
        '>': '&gt;',
        '"': '&quot;',
        "'": '&#039;',
        '/': '&#x2F;'
    };

    return String(text).replace(/[&<>"'\/]/g, char => map[char]);
}

/**
 * Nettoie le HTML en supprimant les balises dangereuses
 * @param {string} html - HTML à nettoyer
 * @returns {string} HTML nettoyé
 */
function sanitizeHtml(html) {
    if (!html || typeof html !== 'string') {
        return '';
    }

    // Créer un élément temporaire
    const temp = document.createElement('div');
    temp.textContent = html;
    return temp.innerHTML;
}

/**
 * Sanitize un texte pour utilisation dans innerHTML
 * Utilise DOMPurify si disponible, sinon fallback sur escapeHtml
 * @param {string} text - Texte à sanitize
 * @returns {string} Texte sécurisé
 */
function sanitizeForInnerHTML(text) {
    if (!text || typeof text !== 'string') {
        return '';
    }

    // Si DOMPurify est disponible, l'utiliser
    if (typeof DOMPurify !== 'undefined') {
        return DOMPurify.sanitize(text, {
            ALLOWED_TAGS: ['b', 'i', 'u', 'strong', 'em', 'br', 'p', 'span', 'div'],
            ALLOWED_ATTR: []
        });
    }

    // Sinon, échapper tout le HTML
    return escapeHtml(text);
}

/**
 * Crée un élément texte sécurisé (pas de HTML)
 * @param {string} text - Texte à afficher
 * @returns {Text} Nœud texte
 */
function createSafeTextNode(text) {
    return document.createTextNode(text || '');
}

/**
 * Définit le contenu texte d'un élément de manière sécurisée
 * @param {HTMLElement} element - Élément cible
 * @param {string} text - Texte à définir
 */
function setSafeText(element, text) {
    if (!element) return;
    element.textContent = text || '';
}

/**
 * Définit le HTML d'un élément de manière sécurisée
 * @param {HTMLElement} element - Élément cible
 * @param {string} html - HTML à définir
 */
function setSafeHTML(element, html) {
    if (!element) return;
    element.innerHTML = sanitizeForInnerHTML(html);
}

// ============================================================================
// VALIDATION DES ENTRÉES
// ============================================================================

/**
 * Valide le format d'un CVE ID
 * @param {string} cveId - ID CVE à valider
 * @returns {boolean} True si valide
 */
function validateCveId(cveId) {
    if (!cveId || typeof cveId !== 'string') {
        return false;
    }
    return /^CVE-\d{4}-\d{4,}$/i.test(cveId);
}

/**
 * Valide le format d'un CWE ID
 * @param {string} cweId - ID CWE à valider
 * @returns {boolean} True si valide
 */
function validateCweId(cweId) {
    if (!cweId || typeof cweId !== 'string') {
        return false;
    }
    return /^CWE-\d+$/i.test(cweId);
}

/**
 * Valide le format d'un CAPEC ID
 * @param {string} capecId - ID CAPEC à valider
 * @returns {boolean} True si valide
 */
function validateCapecId(capecId) {
    if (!capecId || typeof capecId !== 'string') {
        return false;
    }
    return /^CAPEC-\d+$/i.test(capecId);
}

/**
 * Valide le format d'un ATT&CK ID
 * @param {string} attackId - ID ATT&CK à valider
 * @returns {boolean} True si valide
 */
function validateAttackId(attackId) {
    if (!attackId || typeof attackId !== 'string') {
        return false;
    }
    return /^T\d{4}(\.\d{3})?$/i.test(attackId);
}

/**
 * Valide le format d'un D3FEND ID
 * @param {string} d3fendId - ID D3FEND à valider
 * @returns {boolean} True si valide
 */
function validateD3fendId(d3fendId) {
    if (!d3fendId || typeof d3fendId !== 'string') {
        return false;
    }
    return /^(d3f:[A-Za-z0-9]+|D3-[A-Z]+)$/.test(d3fendId);
}

/**
 * Valide une liste de CVE IDs
 * @param {Array} cveList - Liste des CVE
 * @param {number} maxItems - Nombre maximum d'items
 * @returns {object} {valid: boolean, error: string}
 */
function validateCveList(cveList, maxItems = 200) {
    if (!Array.isArray(cveList)) {
        return { valid: false, error: 'cve_list doit être un tableau' };
    }

    if (cveList.length === 0) {
        return { valid: false, error: 'Liste CVE vide' };
    }

    if (cveList.length > maxItems) {
        return { valid: false, error: `Trop de CVE (max: ${maxItems})` };
    }

    const invalidCves = cveList.filter(cve => !validateCveId(cve));
    if (invalidCves.length > 0) {
        return {
            valid: false,
            error: `Format CVE invalide: ${invalidCves.slice(0, 5).join(', ')}`
        };
    }

    return { valid: true, error: '' };
}

/**
 * Sanitize une entrée utilisateur générique
 * @param {*} value - Valeur à sanitize
 * @param {number} maxLength - Longueur maximale
 * @returns {string} Valeur sanitizée
 */
function sanitizeInput(value, maxLength = 1000) {
    if (value === null || value === undefined) {
        return '';
    }

    let text = String(value);

    // Tronquer
    if (text.length > maxLength) {
        text = text.substring(0, maxLength);
    }

    // Échapper HTML
    return escapeHtml(text);
}

// ============================================================================
// PROTECTION CONTRE LES ATTAQUES PAR URL
// ============================================================================

/**
 * Valide une URL pour s'assurer qu'elle est sûre
 * @param {string} url - URL à valider
 * @returns {boolean} True si sûre
 */
function isSafeUrl(url) {
    if (!url || typeof url !== 'string') {
        return false;
    }

    // Whitelist des protocoles autorisés
    const allowedProtocols = ['http:', 'https:', 'mailto:'];

    try {
        const urlObj = new URL(url, window.location.origin);
        return allowedProtocols.includes(urlObj.protocol);
    } catch (e) {
        return false;
    }
}

/**
 * Ouvre une URL de manière sécurisée dans un nouvel onglet
 * @param {string} url - URL à ouvrir
 */
function openSafeUrl(url) {
    if (!isSafeUrl(url)) {
        console.error('[SECURITY] URL non sûre bloquée:', url);
        return;
    }

    const newWindow = window.open(url, '_blank');
    if (newWindow) {
        // Prévenir window.opener vulnerability
        newWindow.opener = null;
    }
}

// ============================================================================
// LOGGING DES ÉVÉNEMENTS DE SÉCURITÉ
// ============================================================================

/**
 * Log un événement de sécurité
 * @param {string} eventType - Type d'événement
 * @param {string} details - Détails
 * @param {string} severity - Niveau de sévérité
 */
function logSecurityEvent(eventType, details, severity = 'INFO') {
    const timestamp = new Date().toISOString();
    const logMessage = `[SECURITY-${severity}] ${timestamp} - ${eventType}: ${details}`;

    if (severity === 'ERROR' || severity === 'CRITICAL') {
        console.error(logMessage);
    } else if (severity === 'WARNING') {
        console.warn(logMessage);
    } else {
        console.log(logMessage);
    }

    // Envoyer au backend si critique
    if (severity === 'CRITICAL' || severity === 'ERROR') {
        try {
            fetch('/api/security/log', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({
                    event_type: eventType,
                    details: details,
                    severity: severity,
                    timestamp: timestamp,
                    user_agent: navigator.userAgent,
                    url: window.location.href
                })
            }).catch(err => console.error('Failed to send security log:', err));
        } catch (e) {
            console.error('Error sending security log:', e);
        }
    }
}

// ============================================================================
// WRAPPERS SÉCURISÉS POUR FONCTIONS DANGEREUSES
// ============================================================================

/**
 * Wrapper sécurisé pour fetch() avec validation
 * @param {string} url - URL à fetcher
 * @param {object} options - Options fetch
 * @returns {Promise} Promesse du fetch
 */
async function secureFetch(url, options = {}) {
    // Valider l'URL
    if (!url.startsWith('/') && !url.startsWith(window.location.origin)) {
        logSecurityEvent('FETCH_BLOCKED', `URL externe bloquée: ${url}`, 'WARNING');
        throw new Error('URL externe non autorisée');
    }

    // Ajouter des headers de sécurité
    options.headers = options.headers || {};
    options.headers['X-Requested-With'] = 'XMLHttpRequest';

    // Timeout par défaut
    const timeout = options.timeout || 30000;
    const controller = new AbortController();
    options.signal = controller.signal;

    const timeoutId = setTimeout(() => controller.abort(), timeout);

    try {
        const response = await fetch(url, options);
        clearTimeout(timeoutId);
        return response;
    } catch (error) {
        clearTimeout(timeoutId);
        throw error;
    }
}

// ============================================================================
// RATE LIMITING CÔTÉ CLIENT
// ============================================================================

/**
 * Rate limiter simple pour prévenir le flood de requêtes
 * Utilise une fenêtre glissante pour tracker les requêtes par endpoint
 */
class RateLimiter {
    /**
     * Constructeur
     * @param {number} maxRequests - Nombre max de requêtes
     * @param {number} timeWindow - Fenêtre de temps en millisecondes
     */
    constructor(maxRequests = 10, timeWindow = 60000) {
        this.maxRequests = maxRequests;
        this.timeWindow = timeWindow;
        this.requests = new Map(); // endpoint -> array de timestamps
        this.cleanupInterval = 300000; // Nettoyage toutes les 5 minutes

        // Démarrer le nettoyage périodique
        this.startPeriodicCleanup();
    }

    /**
     * Vérifie si une requête est autorisée pour cet endpoint
     * @param {string} endpoint - Endpoint/URL de la requête
     * @returns {boolean} True si autorisé
     */
    canMakeRequest(endpoint = 'default') {
        const now = Date.now();
        const cutoff = now - this.timeWindow;

        // Récupérer l'historique pour cet endpoint
        if (!this.requests.has(endpoint)) {
            this.requests.set(endpoint, []);
        }

        let endpointRequests = this.requests.get(endpoint);

        // Nettoyer les anciennes requêtes (fenêtre glissante)
        endpointRequests = endpointRequests.filter(time => time > cutoff);
        this.requests.set(endpoint, endpointRequests);

        // Vérifier la limite
        if (endpointRequests.length >= this.maxRequests) {
            logSecurityEvent(
                'RATE_LIMIT_EXCEEDED',
                `Limite atteinte pour ${endpoint} (${endpointRequests.length}/${this.maxRequests})`,
                'WARNING'
            );
            return false;
        }

        // Ajouter la nouvelle requête
        endpointRequests.push(now);
        this.requests.set(endpoint, endpointRequests);

        return true;
    }

    /**
     * Réinitialise le compteur pour un endpoint
     * @param {string} endpoint - Endpoint à réinitialiser
     */
    reset(endpoint = null) {
        if (endpoint) {
            this.requests.delete(endpoint);
        } else {
            this.requests.clear();
        }
    }

    /**
     * Obtient le nombre de requêtes restantes pour un endpoint
     * @param {string} endpoint - Endpoint à vérifier
     * @returns {number} Nombre de requêtes restantes
     */
    getRemainingRequests(endpoint = 'default') {
        const now = Date.now();
        const cutoff = now - this.timeWindow;

        if (!this.requests.has(endpoint)) {
            return this.maxRequests;
        }

        const endpointRequests = this.requests.get(endpoint)
            .filter(time => time > cutoff);

        return Math.max(0, this.maxRequests - endpointRequests.length);
    }

    /**
     * Obtient le temps d'attente avant la prochaine requête possible
     * @param {string} endpoint - Endpoint à vérifier
     * @returns {number} Millisecondes à attendre (0 si requête possible)
     */
    getWaitTime(endpoint = 'default') {
        if (!this.requests.has(endpoint)) {
            return 0;
        }

        const now = Date.now();
        const cutoff = now - this.timeWindow;
        const endpointRequests = this.requests.get(endpoint)
            .filter(time => time > cutoff);

        if (endpointRequests.length < this.maxRequests) {
            return 0;
        }

        // Temps d'attente = temps jusqu'à ce que la plus ancienne requête sorte de la fenêtre
        const oldestRequest = Math.min(...endpointRequests);
        return Math.max(0, (oldestRequest + this.timeWindow) - now);
    }

    /**
     * Nettoyage périodique des anciennes entrées
     */
    startPeriodicCleanup() {
        setInterval(() => {
            const now = Date.now();
            const cutoff = now - this.timeWindow;

            for (const [endpoint, requests] of this.requests.entries()) {
                const activeRequests = requests.filter(time => time > cutoff);

                if (activeRequests.length === 0) {
                    // Supprimer les endpoints inactifs
                    this.requests.delete(endpoint);
                } else {
                    this.requests.set(endpoint, activeRequests);
                }
            }
        }, this.cleanupInterval);
    }
}

// Instance globale de rate limiter
// 10 requêtes par minute par endpoint
const globalRateLimiter = new RateLimiter(10, 60000);

/**
 * Wrapper pour secureFetch avec rate limiting
 * @param {string} url - URL à fetcher
 * @param {object} options - Options fetch
 * @returns {Promise} Promesse du fetch
 */
async function rateLimitedFetch(url, options = {}) {
    // Extraire l'endpoint pour le rate limiting
    const endpoint = url.split('?')[0]; // Ignorer query params

    // Vérifier rate limit
    if (!globalRateLimiter.canMakeRequest(endpoint)) {
        const waitTime = globalRateLimiter.getWaitTime(endpoint);
        const waitSeconds = Math.ceil(waitTime / 1000);

        throw new Error(
            `Rate limit dépassé pour ${endpoint}. ` +
            `Veuillez attendre ${waitSeconds} seconde(s).`
        );
    }

    // Effectuer le fetch sécurisé
    return secureFetch(url, options);
}

// ============================================================================
// EXPORT DES FONCTIONS GLOBALES
// ============================================================================

// Rendre disponibles globalement
window.SecurityUtils = {
    // XSS Protection
    escapeHtml,
    sanitizeHtml,
    sanitizeForInnerHTML,
    createSafeTextNode,
    setSafeText,
    setSafeHTML,

    // Validation
    validateCveId,
    validateCweId,
    validateCapecId,
    validateAttackId,
    validateD3fendId,
    validateCveList,
    sanitizeInput,

    // URL Safety
    isSafeUrl,
    openSafeUrl,

    // Logging
    logSecurityEvent,

    // Secure Fetch
    secureFetch,
    rateLimitedFetch,

    // Rate Limiting
    RateLimiter,
    globalRateLimiter
};

// Alias pour compatibilité
window.SecurityUtils.validateMitreAttackId = validateAttackId;

console.log('✅ Security utilities (v2.0 sécurisée) loaded - Rate limiting enabled');
