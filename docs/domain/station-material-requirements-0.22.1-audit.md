# Audit 0.22.1 - Exigences materielles et etalonnage

## Perimetre

Cet audit decrit l'etat de `main` 0.22.0 au commit
`e1abe96c388d7f3626415d3a1088dbd8136ab01c`. Il precede toute modification du
domaine, du stockage, des API ou de LAB CONSOLE.

La release doit corriger deux ruptures de sens metier : un montage ne peut
exprimer que des exemplaires immediatement aptes, et le workflow visible de
metrologie ne permet pas d'enregistrer l'evenement d'etalonnage global que le
backend sait deja persister.

## Montage de mesure v2

`StationMeasurementSetupDefinition` porte actuellement :

- un contexte date, lieu et mode d'essai ;
- des `StationAssetBindingDefinition` qui melangent le role logique et
  l'exemplaire physique deja affecte ;
- des connexions entre ports physiques propres au modele selectionne ;
- des selections de caracterisation/correction ;
- un statut de revision `draft`, `ready` ou `superseded`.

La validation structurelle exige au moins deux exemplaires physiques et une
connexion. Le service d'aptitude charge chaque exemplaire du parc, sa revision
de modele et sa metrologie. Cette evaluation est pertinente pour une
utilisation reelle, mais elle intervient trop tot pour decrire le besoin du
montage.

La route globale `GET /api/v1/station-setups/asset-options` renvoie un choix
date et contextualise. LAB CONSOLE n'autorise l'ajout que lorsque
`eligible=true`; les exemplaires bloques sont rendus comme options desactivees.
Le contrat ne peut donc pas distinguer :

1. « ce montage exige CA-001 » ;
2. « CA-001 est affecte et apte le 31 juillet 2026 au labo CEM ».

Cette confusion empêche aussi de definir un role par categorie ou aptitude,
de reporter l'affectation a la preparation d'essai et de construire une
topologie logique avant le choix d'un modele.

## Sources techniques reutilisables

Le domaine equipment fournit deja les sources autoritatives necessaires :

- categories hierarchiques et chemin de categorie ;
- revisions de modeles approuvees et immuables ;
- `MeasurementCapabilityDefinition` avec un `capability_kind` stable ;
- specifications physiques typees ;
- ports, direction, domaine du signal, connecteur, impedance et plages ;
- interfaces de communication ;
- profils de driver et actions qui implementent des aptitudes ;
- parc physique avec revision, disponibilite, lieu et pin exact de modele.

Le nouveau moteur doit les lire depuis Rust. Il ne doit pas reduire une
identite physique a un libelle ou un code inventaire, ni considerer un
`capability_id` local comme portable entre modeles.

## Persistance et compatibilite station

`station.sqlite` separe deja identite, revisions, operations et audit. Les
definitions sont du JSON canonique avec checksum SHA-256. Il n'est pas
necessaire de decomposer le nouveau contrat en tables modifiables : la
revision immuable reste la bonne frontiere d'agregat.

En revanche, la contrainte SQL actuelle et le pointeur
`current_ready_revision_id` ne connaissent pas `qualified`. Une migration
additive doit permettre ce statut sans modifier les migrations publiees. Les
v1/v2 restent lisibles et conservent leur JSON canonique et leur checksum.
Une conversion v2 vers v3 ne peut s'effectuer que sur un nouveau brouillon
derive et doit produire audit et outbox.

## Metrologie existante

Le Local Agent fournit deja :

- `GET /api/v1/metrology/instruments/{asset_id}/calibrations` ;
- `POST /api/v1/metrology/instruments/{asset_id}/calibrations` ;
- `GET /api/v1/metrology/instruments/{asset_id}/status?checked_on=...`.

`record_metrology_calibration` valide les dates, decisions, etats as-found et
as-left, resume d'incertitude et manifeste documentaire. L'ecriture de
`calibration_events`, l'audit, l'operation idempotente et l'outbox sont dans
une transaction. Un certificat duplique pour le meme exemplaire est refuse.

Le backend n'est donc pas le manque principal. LAB CONSOLE ne possede ni type
complet, ni methode client, ni formulaire, ni historique de calibration. Son
formulaire « Ajouter une caracterisation » propose « Certificat
d'etalonnage » comme `source_kind`, ce qui ne signifie pourtant que l'origine
des points mesures. Le panneau de correction affiche en plus « Pret pour un
essai » lorsque les corrections sont disponibles, même si l'étalonnage global
est absent.

## Frontiere de preuve cible

| Concept | Decision portee | Effet sur l'aptitude |
| --- | --- | --- |
| Evenement d'etalonnage | decision globale et validite datee de l'exemplaire | oui |
| Caracterisation | conversion, sensibilite ou reponse mesuree propre a l'exemplaire | non, seule |
| Affectation de correction | valeur revue a appliquer a une exigence de signal | seulement pour la disponibilite de correction |

Une caracterisation issue d'un certificat ne doit jamais creer ou simuler un
evenement d'etalonnage. Un etalonnage ne doit jamais creer silencieusement une
caracterisation ou une affectation de correction.

## HTTP et encodage

`json_response` dans le serveur local renvoie actuellement
`Content-Type: application/json`. Les fichiers HTML, JavaScript et texte
declarent deja correctement UTF-8. Seules les reponses JSON de succes et
d'erreur doivent devenir `application/json; charset=utf-8`, avec des tests sur
les octets francais accentues.

## Architecture cible 0.22.1

Le schema v3 introduit trois niveaux distincts :

1. `StationMaterialRequirementDefinition` decrit le role, la politique de
   selection, le moment d'affectation, la substitution, la metrologie, les
   contraintes et les ports logiques ;
2. `StationMaterialAssignmentDefinition` fige l'exemplaire, sa revision, son
   pin de modele et la correspondance des ports physiques ;
3. l'evaluation contextuelle calcule separement
   `requirement_compatible` et `operationally_eligible`.

Une exigence exacte peut donc etre enregistree avec CA-001 meme si elle est
bloquee. Elle ne devient ni affectation operationnelle, ni montage pret, ni
exception aux regles de service, lieu, reservation, metrologie ou correction.

Le cycle v3 distingue :

- `draft` : definition editable ou incomplete ;
- `qualified` : besoins et topologie logiques coherents, affectations differees
  encore possibles ;
- `ready` : toutes les affectations obligatoires et contrôles contextuels sont
  valides ;
- `superseded` : historique immuable.

## Risques et controles obligatoires

- conversions d'unites inventees : accepter seulement une table explicite de
  conversions de confiance et rendre les autres comparaisons indeterminees ;
- substitutions silencieuses : une exigence exacte ne propose que son
  exemplaire ;
- reponses candidates asynchrones obsoletes : cle de contexte et numero de
  sequence dans LAB CONSOLE ;
- mutations historiques : tests de checksums v2 avant/apres migration et
  derivation ;
- double moteur : aucun matching dans TypeScript ou Python ;
- fausse aptitude globale : afficher separément service, etalonnage,
  corrections et aptitude contextuelle ;
- echec secondaire : conserver le montage ou dossier principal visible et
  proposer une relance ciblee ;
- migration de donnees operateur : toutes les validations utilisent des
  stockages isoles et ne touchent jamais `data/local-agent`.

## Dette explicitement hors release

La release ne lance pas d'acquisition, n'applique aucune correction en
runtime, ne fait ni FFT ni rapport final, n'ajoute ni authentification ni
synchronisation distribuee et n'introduit aucune exception generique aux
regles de securite ou de metrologie.
