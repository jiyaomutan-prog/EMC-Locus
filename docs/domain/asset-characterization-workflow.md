# Caractérisation propre à un matériel

## Rôle et point d’entrée

Le rôle principal est le métrologue. Il entre par `Équipements > Matériels
réels`, sélectionne un exemplaire identifié par son numéro d’inventaire et son
numéro de série, puis ouvre son dossier métrologique.

Le parcours ne part pas d’une bibliothèque de courbes. Il part toujours du
matériel auquel le résultat appartient.

## Objet métier

Une caractérisation est un événement métrologique immuable. Elle décrit une
correction mesurée pour un exemplaire précis et conserve la date, la validité,
la décision, la méthode, l’incertitude et la preuve associée.

Deux résultats sont pris en charge dans cette verticale :

- une conversion temporelle, avec facteur, offset et limites de
  surcharge/écrêtage ;
- une réponse fréquentielle, avec correction d’amplitude et phase optionnelle
  en fonction de la fréquence.

La caractérisation ne modifie ni le modèle d’équipement ni une correction
générique approuvée. Une nouvelle mesure crée un nouvel événement et conserve
l’ancien résultat.

## Parcours normal

1. Le métrologue choisit un matériel réel dans le registre.
2. Il consulte son identification, son modèle approuvé et les caractérisations
   déjà enregistrées.
3. Il choisit `Ajouter une caractérisation`.
4. Il indique si le résultat porte sur des échantillons temporels ou sur un
   spectre en fréquence.
5. Il renseigne la date de mesure, la date de validité, le prestataire ou
   laboratoire, la méthode et la décision.
6. Il saisit le facteur et l’offset, ou importe/saisit le tableau fréquence,
   amplitude et phase optionnelle.
7. Il renseigne l’incertitude et joint la preuve disponible.
8. Il vérifie le résumé puis enregistre l’événement.
9. Le dossier du matériel affiche le résultat, sa validité et son origine. Le
   journal et l’outbox conservent l’opération.

## Décisions demandées

- représentation du signal : temporelle ou fréquentielle ;
- décision métrologique : conforme, non conforme, indéterminée ou non évaluée ;
- opération de correction fréquentielle : ajouter, soustraire, multiplier ou
  diviser ;
- conduite à tenir hors plage temporelle : avertir, refuser ou marquer comme
  écrêté ;
- présence ou non d’une correction de phase.

## Données créées

Chaque événement conserve au minimum :

- un identifiant généré ;
- le numéro d’inventaire du matériel ;
- le type et le nom de la caractérisation ;
- les dates de mesure et de validité ;
- le prestataire, la méthode, la décision et le métrologue ;
- une définition canonique typée et son empreinte technique ;
- un résumé d’incertitude structuré ;
- une référence de certificat ou de feuille de calcul ;
- le document de preuve lorsqu’il est fourni ;
- la justification, l’opération, l’appareil local et la corrélation d’audit.

## États et sélection de la correction applicable

Un événement enregistré est immuable. Il n’a pas de brouillon serveur dans
cette verticale : le formulaire local est vérifié avant l’enregistrement.

Pour un type de correction donné, l’interface distingue :

- `Applicable` : décision conforme et date de validité non dépassée ;
- `À renouveler bientôt` : applicable mais proche de sa date de fin ;
- `Expirée` : date de validité dépassée ;
- `Non conforme` : décision non conforme ;
- `À examiner` : décision indéterminée ou non évaluée.

L’exécution d’essai ne consomme pas encore automatiquement cette correction.
Une future préparation d’essai devra figer explicitement l’identifiant de
caractérisation et son empreinte.

## Erreurs et blocages

L’enregistrement est refusé lorsque :

- le matériel n’existe pas ;
- le nom, les dates, le prestataire, la méthode ou le métrologue manquent ;
- la date de validité précède la date de mesure ;
- une conversion temporelle a des limites incohérentes ou des valeurs non
  numériques ;
- une réponse fréquentielle ne contient pas au moins deux fréquences
  strictement croissantes ;
- une phase est fournie sur certaines lignes seulement ;
- l’unité ou l’opération n’est pas compatible avec le résultat ;
- le document ou son empreinte est invalide ;
- l’identifiant d’opération est rejoué avec un contenu différent.

Les messages de l’interface nomment la donnée métier à corriger. Les chemins
JSON et codes techniques restent réservés au diagnostic.

## Résultat concret

Le métrologue peut ouvrir un câble RF sérialisé, enregistrer ses pertes en dB
entre Fmin et Fmax avec le certificat correspondant, puis retrouver cette
caractérisation et toutes les anciennes mesures dans le dossier du câble.

Le même parcours permet d’enregistrer pour un capteur ou une voie de mesure un
facteur de conversion, un offset et des limites propres à cet exemplaire.
