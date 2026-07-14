# Signaux et corrections

Ce vocabulaire décrit ce que l’équipement fournit au logiciel et la manière
de transformer cette information en une grandeur exploitable. Il remplace le
terme trop large d’« ingénierie de mesure » dans les parcours opérateur.

## Entrées et sorties

Un modèle d’équipement déclare des entrées et des sorties physiques ou
logiques. Un chemin du signal relie ensuite une entrée mesurée à une sortie
interprétée. Par exemple :

    courant dans le conducteur
    -> tension fournie par la pince
    -> échantillons de la voie DAQ
    -> courant calculé en ampères

Ces définitions appartiennent au modèle commun. Une valeur caractérisée pour
un numéro de série précis appartient au registre métrologique. Depuis 0.16.0,
le métrologue peut enregistrer cette valeur, sa validité, son incertitude et sa
preuve dans le dossier du matériel réel.

## Conversion temporelle

Une conversion temporelle s’applique à chaque échantillon d’un signal acquis
dans le temps, par exemple par un numériseur, une carte DAQ ou un oscilloscope.
Le cas usuel est :

    valeur_physique = gain * valeur_brute + offset

Elle décrit donc un gain ou facteur global, un offset, une table de
correspondance éventuelle et les limites de surcharge ou d’écrêtage. Elle ne
compense pas une variation selon la fréquence.

## Réponse fréquentielle

Une réponse fréquentielle s’applique à un niveau ou à un spectre indexé par
la fréquence, par exemple issu d’un récepteur CEM ou d’un analyseur de
spectre. Elle contient au minimum un tableau fréquence/correction d’amplitude
et peut, lorsque la méthode l’exige, contenir une correction de phase.

Chaque composante indique explicitement si elle doit être ajoutée, soustraite,
multipliée ou divisée. La bande exploitable est bornée par les fréquences
minimale et maximale des points contrôlés.

Exemple d’un câble RF :

| Fréquence | Pertes |
| ---: | ---: |
| 10 MHz | 0,2 dB |
| 100 MHz | 1,0 dB |
| 1 GHz | 3,0 dB |

Pour reconstruire le niveau avant le câble, le chemin du signal ajoute les
pertes interpolées au niveau mesuré. La source peut être une fiche technique,
une caractérisation ou, pour un exemplaire réel, un certificat suivi par le
métrologue.

## Alimentation et conditionnement du capteur

Ce terme remplace « excitation » dans l’interface. Il désigne l’énergie ou le
conditionnement nécessaire au capteur : courant IEPE/ICP, tension de pont,
courant constant, alimentation externe ou amplificateur de charge. Ce n’est
pas le signal d’essai appliqué à l’équipement sous test.

## Frontière actuelle

La version 0.16.0 définit, valide et conserve aussi les caractérisations propres
à un numéro de série. Elle ne les applique pas encore à une acquisition réelle,
ne calcule pas de FFT et ne choisit pas automatiquement entre la correction du
modèle et les événements métrologiques disponibles. Une future préparation
d’essai devra figer explicitement la caractérisation retenue et son empreinte.
