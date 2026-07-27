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
- un exemplaire peut rester dans un lieu archivé et son identification reste
  modifiable sans revalidation de ce lieu ;
- déplacer un exemplaire est une commande distincte qui accepte un lieu actif
  ou la suppression explicite de l'affectation ;
- le libellé courant est résolu depuis le registre, tandis que l'audit du
  déplacement conserve les libellés instantanés antérieurs ;
- le registre existe indépendamment des montages prêts ;
- aucun identifiant de lieu n'est dérivé de son libellé ;
- les modifications utilisent une révision attendue.

La commande de déplacement valide la destination et écrit l'exemplaire,
l'audit et l'outbox dans une transaction `BEGIN IMMEDIATE`. Un archivage déjà
commité est donc refusé ; une écriture concurrente sur l'exemplaire produit un
conflit de révision sans preuve partielle.

La 0.22.0 n'introduit pas de gestion de bâtiment, de capacité de salle ni de
calendrier d'occupation supplémentaire.
