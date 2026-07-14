# Corrections propres a un materiel

## Separation des responsabilites

Une correction exploitable pour un numero de serie est composee de deux objets :

1. un evenement de calibration ou de caracterisation immuable, qui porte les
   valeurs et les preuves ;
2. une affectation `AssetCorrectionAssignment`, qui relie cette preuve a une
   exigence precise du modele approuve du materiel.

La preuve contient sa source, la date de mesure, les dates de validite, le
prestataire, la methode, la decision, l'incertitude, les conditions ambiantes,
les valeurs avant/apres reglage, l'indication de reglage, le certificat et son
document adresse par contenu. Sa definition typee et son checksum ne changent
pas apres enregistrement.

L'affectation epingle :

- le materiel et son modele approuve ;
- la revision et le checksum du modele ;
- le chemin et l'exigence de correction ;
- la revision et le checksum de la caracterisation ;
- la source, les conditions et la validite retenues.

## Cycle de revue

```text
draft -> waiting_for_review -> active
                         \-> rejected
waiting_for_review -> draft (demande de correction)
active -> superseded (activation d'un remplacement)
```

Une importation reste donc inutilisable tant qu'elle n'a pas ete soumise,
revue, approuvee et activee. Toutes les transitions exigent une revision
attendue pour la concurrence optimiste ainsi qu'un acteur, un motif et un
`operation_id`. L'activation d'une nouvelle correction remplace atomiquement
l'ancienne correction active pour le meme materiel, le meme chemin, la meme
exigence et les memes conditions. L'ancienne ligne conserve son approbation et
pointe vers son remplacement.

Audit metrologique, journal d'operation et outbox sont ecrits dans la meme
transaction SQLite que la creation ou la transition. L'identite et les droits
reels de l'approbateur restent un futur domaine RBAC ; 0.18.0 ne simule pas une
separation de fonctions qu'il ne peut pas authentifier.
