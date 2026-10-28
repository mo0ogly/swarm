# Revue indépendante du delta 61f82bd

**Verdict : request changes. F1 corrigé ; diagnostic fournisseur corrigé pour les enveloppes attendues ; un bord P2 F2b reste à corriger.**

Candidat propre confirmé : `61f82bda2b4990c3552ec6bf7d76ab25304c02fe`, base `44a86e646c37da503a38e4a687aeb1a50ecb19e1`. Revue indépendante autorisée, en lecture seule sur le candidat. Même périmètre/droits que la revue précédente : aucun nouvel agent, provider réel, processus partagé ou état de mission privée. Rapport unique hors checkout. Réapplication proportionnée de `replan` et `code-reviewer`, déjà lus. L'objectif de cette reprise est de vérifier les deux corrections de la revue précédente ; elle ne requalifie ni n'accepte la recette cumulative native.

## Finding résiduel F2b — [P2] Réserver is_error aux enveloppes fournisseur

Localisation : `internal/engine/assist_run.go:162`.

Le prédicat laisse passer tout événement portant `is_error=true`, quel que soit son type. Un événement de résultat d'outil `{"type":"tool_result","is_error":true,"content":"command failed"}` produit donc `provider_error`, et la variante comportant `error:"Selected model is at capacity. PRIVATE_SECRET"` produit `provider_capacity`. Un échec local d'outil est ainsi déclaré refus fournisseur. Le consommateur `readAssistOutputObserved` conserve ce faux échec puis `runStructuredProviderPolicyClock` le retourne, même si un résultat fournisseur valide arrive ensuite. La précondition de non-classification des outils explicitement demandée et documentée pour F2 n'est pas satisfaite. Le contrat voisin existant `provider_cooldown_test.go:48–50` couvre justement `tool_result/is_error=true` comme événement non-refus.

Contre-exemple exécuté en isolation : la fonction `structuredProviderFailure` et sa constante ont été extraites byte-for-byte du candidat dans un programme Go temporaire, avec quatre entrées. Deux enveloppes fournisseur passent les assertions ; les deux événements outil échouent (exit 1). Aucun appel fournisseur, aucune modification du candidat. Source et helper ont leurs empreintes dans l'original ci-dessous. C'est une exécution directe de la fonction exacte ; je n'ai pas lancé une nouvelle campagne engine/provider complète. Le chemin consommateur est vérifié par lecture de ses appels/retours.

Correction requise : accepter seulement `type=error`, `type=turn.failed`, ou `type=result && is_error=true`. Couvrir les événements `tool_result` top-level avec `is_error=true`, les variantes citées/nested ainsi qu'un résultat fournisseur final après l'outil ; garder les causes reconnues et l'absence de fuite. Le nouveau test `structured_provider_failure_test.go:31` utilise `item.completed` sans `is_error` et ne détecte donc pas ce bord. Le problème de faux refus par top-level is_error existait dans le parseur précédent ; la nouvelle classification rend la fausse capacité explicite et cette correction doit satisfaire sa frontière promise, pas seulement préserver ce défaut hérité.

## Contrôles et conclusions sur le delta

| Critère | Preuve examinée/exécutée | Conclusion |
| --- | --- | --- |
| F1 preview/apply identiques | Diff et chemin complet changeWebRequests : marshal et len(raw)+1 communs avant !preview ; garde de lecture 1 Mio inchangée ; rouge original puis vert source courant examinés | Corrigé au contrat source ; test producteur refus/no-write/relecture PASS |
| F2 causes directes et nested | Diff assist_run/planning_runner ; tests producteur pour message/error.message/error.code/unknown et nonzero exit ; classificateur exact exécuté par reviewer | Corrigé sur enveloppes fournisseur attendues |
| Aucun texte privé exposé par classification | Fonction renvoie exclusivement trois libellés constants ; fixtures PRIVATE_SECRET/URL dans les tests ; programme reviewer vérifie absence PRIVATE_SECRET | PASS local sur ce chemin ; pas un audit de tous les anciens diagnostics du dépôt |
| Pas de classification outil | Nouveau garde + deux contre-exemples top-level outil | FAIL, F2b |
| Pas de fallback/retry implicite | Delta introduit seulement extraction sûre, propagation et borne preview ; aucun changement de route, comptage, réservation, déclenchement ou boucles | PASS de lecture du delta ; aucune nouvelle autonomie réelle déduite |
| Documentation FR/EN | Pages PROVIDER-LIVENESS et WEB-REQUEST-POLICY mises à jour ensemble | Bord no-tool annoncé reste faux tant que F2b ouvert |

Le reçu vert fourni comporte **11 tests de premier niveau / 18 avec sous-cas**. Je l'ai lu et rapproché des sources du delta ; ces exécutions sont celles du producteur, pas de nouveaux tests engine lancés par le reviewer. Les rouges comportent les quatre pertes de causes et le preview de plein historique sur le candidat précédent. Le reviewer a exécuté seulement le contre-exemple Go réduit nouveau, parce que ce cas absent du reçu discrimine la frontière demandée. Les 41/77 contrôles critiques de la revue 44a86e6 restent historiques pour leurs contrats inchangés ; je n'ai pas relancé de suite entière.

Un incident de disque plein a bloqué le build opérateur. À la synchronisation, aucun build/check reviewer n'était actif ; le cache Go reconstructible a été nettoyé par l'opérateur, pas par ce reviewer, et `df` a constaté ensuite 44 Gio libres. Cela modifie la précondition du build, pas le verdict du candidat. Les fichiers d’implémentation examinés sont restés ceux du commit pendant le contre-exemple. Le programme de contre-exemple a pu compiler et s'exécuter après ce changement. Aucune preuve historique supprimée par le reviewer.

## Reprise et libération des sources

Contrôle final : HEAD reste `61f82bda2b4990c3552ec6bf7d76ab25304c02fe`, mais `internal/engine/structured_provider_failure_test.go` est déjà modifié par l’opérateur lors de la finalisation. Cette nouvelle régression en préparation est exclue du candidat et des reçus examinés ici ; le helper exact du contrôle porte l’empreinte du commit. Le checkout était propre au début de la revue.

Sources libérées pour corriger le prédicat et ajouter les variantes de régression. Propriétaire : implémenteur/root. Nouveau commit propre, rouge puis vert sur ce bord, et revue ciblée renouvelée suffisent pour cette correction si les autres entrées pertinentes restent inchangées. Le verdict actuel bloque l'approbation du delta ; il ne vaut pas fin métier ni acceptation native. La recette réelle, ses états/consommations et ses éventuelles nouvelles limites restent sous la responsabilité de l'opérateur ; je n'en affirme aucune acceptation à partir de ses résumés d'avancement.

## Reçus producteur examinés

```json
{
  "t7-review-fixes-red.jsonl": {
    "sha256": "adec031dedc8c343c3b1ee23710a2a57ce06950705bcea5f10c2533c255ee261",
    "top_level_pass": 0,
    "passed_with_subcases": 0,
    "failed_tests": [
      "TestStructuredProviderFailurePreservesSafeCause/message",
      "TestStructuredProviderFailurePreservesSafeCause/nested",
      "TestStructuredProviderFailurePreservesSafeCause/code",
      "TestStructuredProviderFailurePreservesSafeCause/unknown",
      "TestStructuredProviderFailurePreservesSafeCause",
      "TestWebRequestsFullHistoryPreviewAndApplyAgree"
    ]
  },
  "t7-review-fixes-green.jsonl": {
    "sha256": "85fc941c61977851a9c1cc50a5c8251ddbab01514fda25633cf8247a90304c44",
    "top_level_pass": 11,
    "passed_with_subcases": 18,
    "failed_tests": []
  }
}
```

Rapport finalisé 2026-10-13T07:23:16.092805+00:00.

## Original du contrôle reviewer

## Contre-exemple original du classificateur

```json
{
  "candidate": "61f82bda2b4990c3552ec6bf7d76ab25304c02fe",
  "at": "2026-10-13T07:21:30.728493+00:00",
  "method": "exact unchanged structuredProviderFailure + constant extracted; four envelope inputs; isolated Go main, no provider",
  "source_sha256": "6dfe45dd8a8c0bae75afb244e453a8f564ca05bd3c4a98282d346ab836ce758c",
  "helper_sha256": "a10f308d994c1137de46e85e4a57313a2f85cbfabc7d6b6c5a6ec050afb0b2c6",
  "exit": 1,
  "stdout": "provider-capacity expectedNil=false actualError=provider_capacity : mod\u00e8le \u00e0 capacit\u00e9 ; r\u00e9examiner la disponibilit\u00e9 ou choisir explicitement une autre route ; aucune relance automatique / model at capacity pass=true\nprovider-result expectedNil=false actualError=provider_capacity : mod\u00e8le \u00e0 capacit\u00e9 ; r\u00e9examiner la disponibilit\u00e9 ou choisir explicitement une autre route ; aucune relance automatique / model at capacity pass=true\ntool-result expectedNil=true actualError=provider_error : erreur fournisseur non class\u00e9e ; aucune r\u00e9ponse valid\u00e9e ; contenu priv\u00e9 non expos\u00e9 / unclassified provider failure pass=false\ntool-capacity-text expectedNil=true actualError=provider_capacity : mod\u00e8le \u00e0 capacit\u00e9 ; r\u00e9examiner la disponibilit\u00e9 ou choisir explicitement une autre route ; aucune relance automatique / model at capacity pass=false\n",
  "stderr": "exit status 1\n"
}
```

Entrées et assertions exactes du harnais :

```go

func main(){
 cases:=[]struct{name,raw string; wantNil bool}{
  {"provider-capacity",`{"type":"error","message":"Selected model is at capacity. PRIVATE_SECRET"}`,false},
  {"provider-result",`{"type":"result","is_error":true,"result":"Selected model is at capacity. PRIVATE_SECRET"}`,false},
  {"tool-result",`{"type":"tool_result","is_error":true,"content":"command failed"}`,true},
  {"tool-capacity-text",`{"type":"tool_result","is_error":true,"error":"Selected model is at capacity. PRIVATE_SECRET"}`,true},
 }
 failed:=0
 for _,tc:=range cases{var data map[string]any;if err:=json.Unmarshal([]byte(tc.raw),&data);err!=nil{panic(err)};err:=structuredProviderFailure(data);ok:=(err==nil)==tc.wantNil;fmt.Printf("%s expectedNil=%v actualError=%v pass=%v\n",tc.name,tc.wantNil,err,ok);if !ok{failed++};if err!=nil&&strings.Contains(err.Error(),"PRIVATE_SECRET"){panic("privacy leak")}}
 if failed>0{os.Exit(1)}
}

```
