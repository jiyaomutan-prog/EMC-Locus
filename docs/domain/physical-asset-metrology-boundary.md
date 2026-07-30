# Frontière entre parc matériel et métrologie

## Responsabilités

Le **parc matériel** répond à « quel exemplaire utilisons-nous ? ». Il possède
l'identification, le modèle constructeur épinglé, le lieu, la provenance,
l'état de service et la disponibilité.

La **métrologie du parc** répond à « quelles preuves rendent cet exemplaire
apte pour ce contexte de mesure ? ». Elle possède :

- l'exigence et la périodicité d'étalonnage ;
- les événements d'étalonnage et certificats ;
- les caractérisations ;
- les corrections propres à la série ;
- les décisions, validités et éléments d'audit métrologique.

`metrology_asset_dossiers.asset_id` est une référence externe vers l'identité
du parc. Les lectures métrologiques joignent le dossier à
`equipment_db.physical_assets`; elles ne lisent jamais l'archive 0.21.1 pour
composer l'identité courante.

## Conséquences opérationnelles

- un changement de série, lieu ou disponibilité est une commande du parc ;
- un changement d'état de service lancé par l'ancien endpoint métrologie est
  adapté vers la commande du parc et apparaît dans son audit ;
- un étalonnage ou une caractérisation est refusé si aucun dossier rattaché à
  un exemplaire autoritatif n'existe ;
- une calibration propre à une série ne remonte jamais sur le modèle générique ;
- la readiness combine l'identité du parc et les preuves métrologiques sans
  fusionner leurs sémantiques.

Les mathématiques de correction, l'acquisition et l'application numérique des
corrections ne sont pas étendues par cette release.

## Statut métrologique daté

Le statut affiché n'est jamais déduit dans l'interface à partir de la seule
échéance. Le cœur calcule un verdict pour une date civile `checked_on` à partir
de l'exigence, de la dernière décision, des dates d'étalonnage et du seuil
d'attention :

- `valid`, `due_soon`, `expired` ;
- `missing`, `nonconforming`, `indeterminate` ;
- `not_required` ;
- `unavailable` lorsque la source métrologique ne peut pas être lue.

Le contrat contient aussi la date contrôlée, la décision, `calibrated_at`,
`due_at`, le seuil, le caractère bloquant, une explication et des codes de
raison. Une échéance égale à `checked_on` est `due_soon`; elle n'est expirée
que si elle est strictement antérieure. Les comparaisons portent sur des dates
civiles `YYYY-MM-DD`, sans conversion par le fuseau du navigateur.
L'exigence est `null` uniquement lorsque la source est indisponible : le parc
ne transforme pas une panne de lecture en fausse exigence métier.

Le parc, le sélecteur de montage et l'instantané de préparation consomment ce
même verdict. Une panne de `metrology.sqlite` conserve l'identité physique et
retourne `metrology.status = unavailable`; elle ne fait jamais échouer toute
la liste du parc.
