# Parcours operateur des corrections

## Depuis le modele

Dans `Catalogue equipements`, l'onglet `Entrees, sorties et corrections`
decrit les chemins physiques et les corrections attendues. Le formulaire pose
des questions concretes : objet de la correction, dependance a la frequence,
operation, besoin d'une valeur par exemplaire, repli nominal et caractere
bloquant.

Le modele ne recoit jamais un certificat propre a un numero de serie.

## Depuis le materiel reel

Dans `Materiels reels`, le dossier affiche immediatement :

- l'identite et le numero de serie ;
- l'etat de service, l'etat d'etalonnage et le modele epingle ;
- chaque correction requise et son etat ;
- `Pret pour un essai` ou `Non pret pour un essai` ;
- l'action suivante sur chaque correction manquante.

`Mesurer cette correction` ouvre un seul parcours contextualise : origine,
valeurs, apercu graphique, validite, preuve, puis revue. Le brouillon peut etre
soumis, renvoye pour correction, refuse ou approuve et active. La bibliotheque
technique globale n'est pas un prealable au travail du metrologue.

Lorsqu'une valeur nominale et une valeur etalonnee existent, la valeur propre au
materiel est affichee comme selectionnee ; la valeur nominale reste visible mais
explicitement non selectionnee.

## Preuves visuelles

Les captures de la release sont sous `docs/ux/0.18.0/screenshots/` pour
`1440 x 900` et `1280 x 720`. Elles couvrent l'exigence du modele, le blocage,
l'import, la revue, l'etat pret et la comparaison nominale/etalonnée.
