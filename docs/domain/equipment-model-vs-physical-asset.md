# Modèle constructeur et exemplaire du parc

## Deux objets différents

Un **modèle constructeur** décrit un type réutilisable d'équipement. Il porte
le fabricant, le nom de modèle, la variante, la catégorie, les capacités, les
ports, les interfaces, la documentation générique et les exigences de
correction. Il est révisionné et un exemplaire épingle exactement une version
approuvée et son empreinte.

Un **exemplaire du parc** décrit ce qui existe réellement au laboratoire. Il
porte un code inventaire unique, un numéro de série facultatif, un éventuel
part number, une provenance, un lieu, un état de service, une disponibilité et
des notes. Son identifiant interne n'est pas une donnée opérateur.

Le modèle ne porte jamais de code inventaire, de série, de lieu, de certificat
d'étalonnage ni de correction propre à un exemplaire. Le core rejette ces
champs même lorsqu'ils sont dissimulés dans les maps d'extension.

## Épinglage de version

À la création d'un exemplaire, le Local Agent résout la version approuvée
courante du modèle et enregistre :

- l'identité stable du modèle ;
- l'identité exacte de sa version ;
- son empreinte SHA-256 ;
- un instantané lisible du fabricant, du modèle, de la variante et du chemin
  de catégorie.

Une nouvelle version du modèle ne repointe jamais les exemplaires existants.
L'instantané garde leur contexte historique même si le catalogue évolue.

## États séparés

L'**état de service** répond à la question « cet exemplaire est-il apte ? » :
utilisable, utilisation restreinte, en maintenance, hors service ou retiré.

La **disponibilité administrative** répond à la décision manuelle du parc :
disponible ou indisponible avec un motif. Un exemplaire en maintenance, hors
service ou retiré est nécessairement indisponible.

L'**utilisation opérationnelle** répond à la question « que mobilise réellement
cet exemplaire maintenant ? ». Réservation, présence dans un montage et essai
actif sont dérivés des enregistrements autoritatifs de ces workflows. Ils ne
peuvent pas être saisis dans la fiche du parc. Ces dimensions ne sont pas
interchangeables et les transitions manuelles sont protégées par une révision
optimiste.
