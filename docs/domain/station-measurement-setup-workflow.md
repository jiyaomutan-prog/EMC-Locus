# Préparation d'un montage de mesure

## Rôle et point d'entrée

Le rôle principal est le technicien ou l'ingénieur d'essai qui prépare un poste
de mesure. Il entre dans **Locus Test Station**, ouvre l'espace `Préparation du
poste`, puis crée ou reprend un montage de mesure.

Le parcours ne part ni des tables de la base ni d'une bibliothèque de contrats
techniques. Il part du montage physique que l'opérateur doit réaliser sur son
poste.

## Objet métier

Un **montage de mesure** décrit les matériels réels présents sur le poste, le
rôle de chacun, les connexions entre leurs ports et les caractérisations
métrologiques retenues pour interpréter le signal.

La transformation retenue dépend de la représentation des données :

- une **conversion temporelle** applique notamment gain, offset et limites à
  chaque échantillon ;
- une **réponse fréquentielle** compense l’amplitude, et si nécessaire la
  phase, pour chaque fréquence du spectre.

`Scaling`, `courbe d’ingénierie` et `excitation` ne sont pas des décisions
opérateur dans ce parcours. L’alimentation ou le conditionnement d’un capteur
reste une propriété physique explicite de la chaîne.

Un montage fige les références des modèles et des caractérisations utilisées.
Il ne modifie jamais le catalogue des modèles ni le dossier métrologique d'un
matériel.

Cette première verticale prépare et valide le montage. Elle ne pilote pas les
instruments et n'acquiert aucune donnée.

## Parcours normal

1. L'opérateur crée un montage et lui donne un nom de laboratoire, par exemple
   `Mesure des pertes du câble EUT`.
2. Il précise le poste utilisé et la date prévue d'utilisation.
3. Il ajoute des matériels réels depuis le registre métrologique. Le numéro
   d'inventaire, le numéro de série, l'état de service et l'échéance
   d'étalonnage sont affichés ensemble.
4. Il donne à chaque matériel un rôle lisible dans ce montage, par exemple
   `Récepteur`, `Câble de mesure`, `Capteur` ou `Voie DAQ`.
5. Il relie un port de sortie ou de traversée à un port d'entrée ou de
   traversée. L'interface propose les ports issus du modèle approuvé du
   matériel et affiche le connecteur, le domaine de signal et la plage de
   fréquence utiles.
6. Pour un matériel dont la réponse propre doit être compensée, il sélectionne
   une caractérisation applicable à la date du montage, par exemple les pertes
   mesurées du câble réel.
7. Il enregistre le brouillon. Le Local Agent vérifie la structure, les
   matériels, les ports, les compatibilités et l'aptitude métrologique.
8. L'interface présente les blocages et avertissements en langage métier, en
   indiquant le matériel ou la connexion concernés.
9. Lorsque aucun blocage ne subsiste, l'opérateur marque le montage `Prêt à
   câbler`. Cette version devient immuable.
10. Pour faire évoluer un montage prêt, l'opérateur crée un nouveau brouillon à
    partir de celui-ci. L'ancienne version reste consultable.

## Décisions demandées

- le nom du montage et le poste physique utilisé ;
- la date à laquelle l'aptitude des matériels doit être évaluée ;
- les matériels réels présents et leur rôle dans le montage ;
- le port relié de chaque côté d'une connexion ;
- la caractérisation propre au matériel à retenir lorsqu'une correction est
  nécessaire ;
- l'acceptation ou la correction des avertissements avant de déclarer le
  montage prêt.

## Données créées ou modifiées

Le montage conserve :

- une identité stable et un numéro de version déterministe ;
- son nom, son poste et sa date d'utilisation prévue ;
- chaque matériel sélectionné, son rôle et la référence figée de son modèle
  approuvé ;
- chaque connexion avec ses deux ports ;
- chaque caractérisation retenue avec sa référence et son empreinte ;
- le verdict de préparation et ses causes détaillées ;
- l'auteur, la justification, les dates et la traçabilité d'opération ;
- les événements d'audit et d'outbox associés.

Les libellés métier sont conservés pour la lecture. Les identifiants techniques
et empreintes sont générés ou résolus par le logiciel.

## États

- `Brouillon` : le montage peut être remplacé avec contrôle de concurrence ;
- `Prêt à câbler` : le montage a passé le contrôle et devient immuable ;
- `Remplacé` : une version prête plus récente existe pour la même identité.

Un ancien brouillon ne peut pas écraser une modification plus récente. Une
version prête n'est jamais modifiée sur place.

## Contrôles et blocages

Le contrôle bloque notamment lorsque :

- un matériel réel n'existe plus ou sa référence de modèle n'est plus
  cohérente ;
- un matériel est hors service, retiré ou soumis à une restriction incompatible
  avec le montage ;
- un étalonnage requis est absent ou expiré à la date prévue ;
- un port n'existe pas dans la révision de modèle figée ;
- deux ports ont des directions incompatibles ;
- les domaines de signal, connecteurs, impédances ou plages de fréquence sont
  incompatibles ;
- une entrée est alimentée par plusieurs connexions sans que le modèle ne
  l'autorise ;
- une connexion forme une boucle non déclarée ;
- une caractérisation appartient à un autre matériel, n'est plus applicable ou
  ne correspond pas au type de correction attendu ;
- le montage ne contient aucun chemin de signal exploitable ;
- une modification concurrente a remplacé le brouillon affiché.

Un avertissement non bloquant peut signaler une information physique absente,
par exemple un connecteur non renseigné ou une impédance non définie. Il ne doit
jamais transformer une incompatibilité connue en simple avertissement.

## Résultat concret attendu

L'opérateur peut préparer un chemin `antenne réelle -> câble RF réel ->
récepteur réel`, choisir la caractérisation de pertes du câble correspondant à
son numéro de série, constater l'aptitude métrologique de chaque matériel et
obtenir un montage prêt à câbler conservé avec son audit.

Le même contrat doit pouvoir représenter ensuite une chaîne temporelle
`capteur -> conditionneur -> voie DAQ`, sans mélanger la préparation physique
avec l'acquisition ou le traitement des données.
