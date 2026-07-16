# Cohérence du démarrage d'un essai planifié

## Décision métier protégée

Le passage d'un créneau **Confirmé** à **En cours** est une décision
transversale. Il ne suffit pas qu'une préparation ait été prête à un instant
antérieur : la preuve vue par l'opérateur doit encore être la préparation
courante et ses entrées doivent encore être aptes au moment de la transition.

La commande de démarrage fournit donc :

- la révision attendue du créneau ;
- l'identifiant de la révision de préparation affichée ;
- l'empreinte canonique de cette préparation ;
- l'identifiant d'opération utilisé pour l'idempotence.

Le contexte figé compare aussi l'identifiant stable du lieu du créneau à celui
du montage. Les libellés sont conservés comme instantanés lisibles mais ne
participent jamais à la décision : un renommage reste compatible, tandis qu'un
autre identifiant avec le même libellé est bloquant. Une preuve 0.21.0 sans
identité produit un blocage explicite au lieu d'inférer un lieu depuis le texte.

L'identifiant et l'empreinte de préparation font partie du fingerprint de la
commande. Réutiliser un identifiant d'opération avec une preuve différente est
un conflit de rejeu.

## Frontière de cohérence locale

Le Local Agent ouvre `projects.sqlite` et lui attache :

- `sync.sqlite` ;
- `test_definitions.sqlite` ;
- `station.sqlite` ;
- `metrology.sqlite` ;
- `equipment.sqlite`.

Ces bases doivent utiliser un journal de rollback compatible avec la politique
multi-SQLite du dépôt (`DELETE`, `TRUNCATE` ou `PERSIST`). Le démarrage ouvre
ensuite une transaction `BEGIN IMMEDIATE` avant toute validation finale.

Le verrou réservé porte sur toutes les bases attachées. Les repositories de
lecture existants peuvent encore ouvrir une connexion dédiée pour relire une
méthode, un montage, un matériel ou un étalonnage : aucune écriture concurrente
sur ces bases ne peut toutefois s'engager avant la fin de la transaction de
démarrage. Les lectures et le compare-and-set projets appartiennent donc à une
même période sérialisée contrôlée par le Local Agent.

Cette solution est locale au processus de stockage SQLite. Elle ne remplace pas
une future coordination distribuée et suppose que toutes les écritures métier
respectent la politique de journal et passent par le Local Agent.

## Validation finale

Dans la transaction immédiate, le service relit et vérifie :

1. le projet et le créneau ;
2. l'état **Confirmé** et la révision attendue du créneau ;
3. le pointeur vers la préparation courante ;
4. l'identifiant, l'empreinte, le verdict et le contenu canonique de cette
   préparation ;
5. la révision et l'empreinte de la méthode figée ;
6. la révision et l'empreinte du montage figé ;
7. les matériels, leur état de service, leur étalonnage et les preuves
   métrologiques nécessaires ;
8. le verdict recalculé dans le contexte actuel du créneau.

La mise à jour du créneau est un compare-and-set qui répète la comparaison de
la révision du créneau, du pointeur de préparation et de son empreinte. La même
transaction écrit ensuite :

- le statut **En cours** ;
- l'audit citant exactement la révision et l'empreinte autorisantes ;
- l'opération d'outbox.

Une modification contrôlée entre la validation et le compare-and-set produit
`planned_test_preparation_changed_before_start` et le message opérateur :

> La préparation de l'essai a changé pendant le démarrage. Vérifiez-la de
> nouveau.

La transaction est alors annulée et le créneau reste **Confirmé**.

## Preuves automatisées

Les tests Rust utilisent des hooks transactionnels déterministes, sans délai
arbitraire, pour vérifier :

- l'installation d'une nouvelle préparation bloquée comme révision courante
  entre validation et compare-and-set ;
- le changement du pointeur courant ;
- le changement de révision du créneau ;
- le refus d'un writer station ou métrologie pendant la validation ;
- l'annulation complète des mutations simulées après conflit ;
- le rejeu identique d'un démarrage réussi ;
- le refus du même identifiant d'opération avec une autre empreinte ;
- la révision et l'empreinte exactes conservées dans l'audit.
