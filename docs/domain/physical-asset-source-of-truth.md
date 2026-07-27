# Source de vérité des exemplaires du parc

## Décision

Depuis la 0.22.0, l'unique source modifiable de l'identité d'un exemplaire est :

`equipment.sqlite / physical_assets`

Le domaine equipment/fleet possède l'identité, le lien constructeur, les
instantanés lisibles, le lieu, la provenance, l'état de service et la
disponibilité administrative. L'utilisation opérationnelle est une projection
datée issue des réservations, montages et essais réels ; elle n'est pas un état
manuel du parc. Les écritures passent par des transactions `BEGIN IMMEDIATE`
incluant contrôle de concurrence, mutation, audit et outbox.

Les commandes d'écriture respectent deux responsabilités :

- **Modifier l'identification** ne touche qu'au code inventaire, au numéro de
  série, à la référence fabricant, à la provenance et aux notes ;
- **Déplacer l'exemplaire** valide une destination active ou retire
  explicitement l'affectation de lieu.

Cette séparation permet de corriger une fiche située dans un lieu archivé sans
transformer l'édition en nouvelle affectation implicite.

Le registre des lieux est également possédé par equipment :

`equipment.sqlite / laboratory_locations`

## Migration 0.21.1

La migration SQL métrologie 11 renomme `instruments` en
`legacy_instruments_0_21_1`, rend cette archive non modifiable et recâble les
preuves vers `metrology_asset_dossiers`. La migration equipment 8 ajoute la
preuve de rapprochement. Après application de toutes les migrations, un
coordinateur transactionnel attachant equipment, metrology et sync :

1. importe chaque ancienne identité avec le même `asset_id` ;
2. utilise cet identifiant comme code inventaire de migration ;
3. conserve série, part number, fabricant, modèle, catégorie et états ;
4. conserve les anciennes références de modèle dans la preuve de migration ;
5. ne déclare un lien modèle résolu que si l'identité, la version et l'empreinte
   existent et concordent ;
6. écrit audit et outbox ;
7. clôt la migration dans `equipment_cross_domain_migrations`.

Le rejeu lit cette clôture et ne duplique aucune ligne. Un conflit d'identité
ou de code inventaire arrête la transaction entière.

## Adaptateur temporaire

`POST /api/v1/metrology/instruments` reste temporairement disponible pour les
anciens clients. Il crée l'identité dans `physical_assets` et le dossier dans
`metrology_asset_dossiers` sous une seule transaction attachée. Une référence
constructeur fournie par le client n'est acceptée que si sa version et son
empreinte contrôlées existent. Sinon l'exemplaire est marqué à rapprocher.

La nouvelle interface et les nouveaux clients doivent utiliser
`POST /api/v1/fleet/assets`. L'adaptateur sera supprimé lorsque les clients Qt
historiques ne l'utiliseront plus.
