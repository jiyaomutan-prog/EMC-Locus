# Registre des lieux du laboratoire

## Contrat minimal

Un lieu possède une identité stable, un libellé, une description facultative,
un état actif ou archivé, une révision et des dates de création et mise à jour.
Chaque écriture produit audit et outbox sous la même transaction.

Le libellé est unique sans distinction de casse. Il peut être renommé sans
changer l'identité. Les exemplaires et futurs enregistrements historiques
conservent également un instantané lisible.

## Règles

- un lieu actif peut être affecté à un exemplaire, un montage ou une réservation ;
- un lieu archivé reste lisible dans l'historique ;
- un lieu archivé ne peut plus être choisi pour une nouvelle affectation ;
- le registre existe indépendamment des montages prêts ;
- aucun identifiant de lieu n'est dérivé de son libellé ;
- les modifications utilisent une révision attendue.

La 0.22.0 n'introduit pas de gestion de bâtiment, de capacité de salle ni de
calendrier d'occupation supplémentaire.
