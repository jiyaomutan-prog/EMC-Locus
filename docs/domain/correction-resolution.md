# Resolution des corrections

## Entree contextuelle

Le resoluteur travaille pour un materiel, une date d'utilisation, un contexte
d'execution et des conditions telles que `polarization=horizontal`. Il ne
calcule pas un statut global fige du materiel.

Ordre deterministe pour chaque exigence applicable :

1. correction propre au materiel, active et valide a la date demandee ;
2. correction propre au materiel approuvee sans date de fin, si la politique
   l'autorise ;
3. valeur nominale approuvee du modele, uniquement si la politique, la qualite
   et le contexte l'autorisent ;
4. aucune correction disponible.

Les candidats de meme rang sont tries par date d'affectation decroissante puis
par identite, afin de garantir le meme resultat apres redemarrage. Les
conditions de l'exigence et de l'affectation doivent correspondre au contexte
demandé. Une correction horizontale ne satisfait donc pas une demande
verticale.

## Regles de repli

- `asset_required` interdit tout repli nominal ;
- en contexte `accredited`, seule une politique `model_value_only` peut retenir
  la valeur du modele ;
- `simulation_only` n'est utilisable qu'en contexte `simulation` ;
- un repli nominal autorise est visible et accompagne d'un avertissement ;
- une exigence obligatoire manquante, expiree, en brouillon ou en attente de
  revue rend le rapport non pret ;
- l'absence d'une exigence facultative produit un avertissement non bloquant.

Le resultat indique la source selectionnee, la revision, le checksum, la
validite, la raison, le repli eventuel et le caractere bloquant. LAB CONSOLE
traduit ces preuves en francais et masque les codes techniques hors diagnostic.

## Limite 0.18.0

La resolution est une previsualisation d'aptitude. Elle ne cable pas une station,
n'applique pas numeriquement la correction a des echantillons ou a un spectre et
ne remplace pas le pre-vol complet d'une campagne.
