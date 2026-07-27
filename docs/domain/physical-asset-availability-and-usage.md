# Disponibilite administrative et utilisation operationnelle

## Deux faits distincts

Le domaine parc ne permet de saisir manuellement que la **disponibilite
administrative** d'un exemplaire :

- `available` : disponible administrativement ;
- `unavailable` : indisponible, avec un motif obligatoire.

Les valeurs `reserved`, `assigned_to_setup` et `in_test` ne sont pas des
decisions administratives. Elles decrivent une utilisation issue d'autres
workflows et ne peuvent donc pas etre inventees par un operateur du parc.

## Projection calculee

Chaque lecture d'un exemplaire produit `operational_usage` pour l'instant
demande (`GET /api/v1/fleet/assets?at=<RFC3339>`). Cette projection n'est pas
persistee comme source de verite. Elle expose :

- l'etat calcule ;
- l'instant d'evaluation ;
- les preuves avec type et identifiant de source, libelle lisible, intervalle,
  motif et caractere bloquant.

La priorite deterministe est : indisponibilite administrative ou technique,
essai actif, reservation couvrant l'instant, reference par un montage pret,
puis disponible. La simple reference par un montage est informative et ne
bloque pas la selection en l'absence d'une regle d'allocation exclusive.

Les sources lues sont les executions de mesure actives, les preparations et
creneaux d'essai planifies, et les revisions de montage pretes. Une source
secondaire indisponible ajoute une preuve non bloquante d'indisponibilite de la
source ; elle ne masque jamais l'identite de l'exemplaire.

## Migration 0.22.0

La migration equipment `0009_administrative_availability.sql` ajoute les
colonnes administratives et conserve l'ancien etat dans
`legacy_availability_evidence_json`.

- `available` devient administrativement disponible ;
- `unavailable` devient administrativement indisponible avec un motif ;
- `reserved`, `assigned_to_setup` et `in_test` deviennent administrativement
  disponibles, tandis que leur valeur historique reste dans la preuve de
  migration.

Aucune reservation ni execution active n'est deduite de cette valeur legacy.
L'utilisation courante est toujours recalculee depuis les enregistrements
autoritaires.

## Compatibilite temporaire

La colonne `availability_state` et la route
`POST .../transitions/availability` restent des miroirs de compatibilite pour
les clients 0.21.x. Elles n'acceptent plus que `available` ou `unavailable` et
ne peuvent creer aucun fait operationnel. Les nouveaux clients utilisent
`administrative_availability`, `administrative_unavailability_reason` et la
route `transitions/administrative-availability`. Ce miroir sera supprime avec
les derniers clients historiques.

L'eligibilite complete pour un montage ou une preparation est un contrat
backend distinct. Elle combine cette projection avec la metrologie, le lien de
modele, le lieu et les exigences du contexte cible ; elle n'est jamais decidee
par un filtre frontend seul.
