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
