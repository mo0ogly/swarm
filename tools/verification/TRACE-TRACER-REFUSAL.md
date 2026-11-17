# Refus du traceur, primaire inconnu

`run_tracer_refusal(out, admission, strace, python)` conserve le refus propre du
traceur demandé avec une destination sous un fichier régulier. Un contrôle frais
code0 précède cette faute et une reprise fraîche code0 la suit, via le noyau
`trace_process_witness` inchangé. Le primaire reste toujours `UNKNOWN` et la
capture incomplète : absence de trace ou stdout vide ne prouve pas non-exécution.

Le caller doit fournir stockage/FD/admission qualifiés, garde20Gio hôte/sorties,
leaf privé neuf0700, umask077 et pins sources/helpers/outils stables. L'allocation
de chaîne strace vient des seuls argv/env connus. Aucun environnement parent
n'est hérité. Les fichiers sont exclusifs0600 et conservés, sans retry ni timeout.

Le reçu natif décrit le traceur directement lancé. Seule sa ValueError numérique
attendue est acceptée, avec code réel strictement positif, aucun signal/autre
issue, publication/post-admission réussies et canaux intacts. Stdout/stderr sont
des observations conservées, sans interprétation de mots d'erreur. Bloqueur et
reçus de la faute doivent rester identiques après reprise. Si une validation,
reprise ou publication ultérieure échoue, un groupe conserve les erreurs et un
carrier natif avec `earlier_expected_tracer_refusal` et `following_failure` ;
le refus antérieur n'est pas perdu derrière le diagnostic plus récent.
Les interruptions `BaseException` sont conservées dans un `BaseExceptionGroup`
et propagées, jamais converties en succès ou acceptées comme refus attendu.

La recette `recipes/trace-tracer-refusal-inert.json` vérifie uniquement ce dernier
contrat par mocks : témoin sain, hash stdout injecté EIO après refus1 synthétique,
reprise fraîche et conservation du carrier. `SWARM_TRACE_TEST_PARENT` explicite
est requis. Aucun enfant ni véritable défaut de stockage/strace n'y est exécuté.
Une seconde étape injecte une interruption après le refus connu et vérifie la
conservation du même carrier ainsi que de l'interruption dans son groupe.
Le témoin réel exige une allocation distincte sous les préconditions du caller.

Résultat : complete_build_qualification=false et behavioral_admission=false.
Pas de Cargo, provider, Swarm installé, corps Rust, sémantique générale de trace,
arbre arbitraire ou isolation OS. Le code réel du primaire reste non établi.
