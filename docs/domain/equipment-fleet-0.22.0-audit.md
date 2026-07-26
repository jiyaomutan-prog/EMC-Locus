# Audit 0.22.0 - Parc matériel et lisibilité métier

## Statut et périmètre

Cet audit décrit l'état de la release `0.21.1` au commit
`471e656ab007c1d627456a8dc90d9e3064074f35` et fixe la cible de la verticale
`0.22.0`. Il précède volontairement toute modification du domaine, du stockage,
des APIs ou de LAB CONSOLE.

Le constat principal est le suivant : le catalogue sait déjà représenter un
modèle constructeur révisionné, mais le seul enregistrement d'un exemplaire
physique est encore la ligne `instruments` du domaine métrologique. L'identité
du parc, son état de service et sa future localisation sont donc possédés par le
mauvais domaine.

## Cartographie 0.21.1

### Modèle constructeur

Le domaine `equipment` possède un agrégat réutilisable et révisionné :

- `EquipmentModelDefinition` dans
  `crates/emc-locus-core/src/equipment.rs` porte la classification physique,
  les caractéristiques, les ports, les chemins de signal, les interfaces de
  communication, les capacités et les exigences de correction génériques ;
- `equipment_model_identities` sépare l'identité stable des versions de
  contenu ;
- `equipment_model_revisions` garantit une numérotation déterministe, un parent
  explicite, un JSON canonique, une empreinte SHA-256 et l'immutabilité après
  soumission ;
- `equipment_model_classification_summaries` et les tables associées servent
  les recherches sans relire le JSON ;
- les catégories et formulaires hérités sont stockés dans
  `equipment_categories`, `equipment_field_definitions` et
  `equipment_category_field_rules` ;
- l'API `/api/v1/equipment-models` et le client Python exposent ce cycle de vie.

Cette base est conservée. Elle doit toutefois interdire explicitement les
champs réservés à un exemplaire, y compris lorsqu'ils sont dissimulés dans
`custom_field_values` ou `metadata`.

### Exemplaire physique et métrologie

Le domaine `metrology` mélange actuellement trois responsabilités :

1. identité physique dans `instruments` ;
2. politique et événements métrologiques ;
3. corrections propres à un exemplaire.

`InstrumentRecord`, `StoredInstrument`, `RegisterInstrumentInput`,
`MetrologyInstrument` et `RegisterMetrologyInstrumentInput` représentent en
réalité un exemplaire du parc. Ils exigent un numéro de série, utilisent
`asset_id` comme identifiant interne et comme numéro d'inventaire affiché, et
portent également fabricant, modèle, catégorie, capacités, part number et état
de service.

Les preuves métrologiques sont correctement rattachées par `asset_id` :

- `calibration_records` pour l'historique importé ;
- `calibration_events` pour les nouveaux étalonnages ;
- `instrument_documents` pour les certificats et documents ;
- `asset_characterization_events` pour les conversions temporelles et réponses
  fréquentielles propres à l'exemplaire ;
- `asset_correction_assignments` pour les corrections revues et actives ;
- `metrology_audit_events`, le journal d'opérations et l'outbox de
  synchronisation.

Ces preuves doivent rester métrologiques. Seule l'identité physique doit sortir
de ce domaine.

### Montages et préparation d'essai

`StationAssetBindingDefinition` référence déjà un `asset_id`, sa révision et la
version exacte du modèle constructeur. Un montage n'accepte donc pas
intentionnellement un simple modèle. Sa validation charge cependant cet objet
dans `metrology.sqlite` avec `load_instrument`.

`PreparedStationAssetSnapshot` conserve l'identité et les références figées.
Dans `planned_test_preparation_service.rs`, `inventory_code` est néanmoins
fabriqué avec `binding.asset_id`; ce n'est pas un vrai code inventaire. Le
numéro de série est aussi remplacé par `Non disponible` quand l'instrument ne
peut pas être chargé, alors que l'absence volontaire de numéro de série n'est
pas modélisable.

La préparation utilise encore le nom
`PlannedTestInstrumentAssignment` pour une affectation qui peut concerner un
câble, un accessoire, un logiciel ou une installation. Le contrat sémantique est
donc plus étroit que le besoin réel.

### Lieux

Il n'existe aucun registre autonome des lieux :

- les montages stockent un couple libre
  `laboratory_location_id` / `laboratory_location_label` dans leur définition ;
- le planning stocke le même couple et conserve les anciens libellés sans ID ;
- `projectApi.listLaboratoryLocations` appelle `/api/v1/station-setups`, ne
  garde que les versions prêtes et déduit une `Map` de lieux ;
- un lieu ne peut donc pas exister avant un montage prêt ;
- le serveur reçoit encore l'ID et le libellé depuis le client pour créer un
  montage ou un créneau ;
- aucune règle centrale n'empêche l'utilisation d'un lieu archivé ni la
  divergence de deux libellés pour le même ID.

La correction 0.21.1 a sécurisé les conflits de planning par identité stable,
mais elle n'a volontairement pas créé la source de vérité de cette identité.

## Identités dupliquées ou modifiables au mauvais endroit

| Information | Écriture 0.21.1 | Problème | Propriétaire 0.22.0 |
| --- | --- | --- | --- |
| `asset_id` | `metrology.instruments` | sert aussi de code inventaire visible | parc, ID interne masqué |
| code inventaire | absent | reconstruit depuis `asset_id` | parc, unique et modifiable sous concurrence |
| fabricant | modèle + instrument | deux valeurs indépendantes | modèle; instantané figé dans le parc |
| nom de modèle | modèle + instrument | deux valeurs indépendantes | modèle; instantané figé dans le parc |
| variante | modèle seulement | instantané physique absent | modèle; instantané figé dans le parc |
| numéro de série | instrument, obligatoire | interdit câbles, logiciels et installations non sérialisés | parc, optionnel |
| part number | instrument | mêlé au dossier métrologique | parc |
| famille/catégorie | modèle + instrument | liens facultatifs et valeurs divergentes possibles | modèle; instantané figé dans le parc |
| capacités | modèle + instrument | copie librement modifiable en métrologie | modèle révisionné; lecture via version figée |
| état de service | instrument | possédé par la métrologie | parc |
| disponibilité | ancien champ instrument | confond réservation et aptitude | parc, distincte de l'état de service |
| propriétaire/source | champ de formulaire au scope modèle | une propriété d'exemplaire contamine le modèle | parc |
| référence interne | champ de formulaire au scope modèle | ressemble à un code inventaire de modèle | supprimer du modèle ou reformuler en référence de catalogue |
| lieu | montage + planning | pas de source de vérité, libellé client libre | registre des lieux; instantanés historiques |
| politique d'étalonnage | instrument | appartient au dossier métrologique, pas à l'identité physique | dossier métrologique référencé par `asset_id` |

Les références de version, empreintes et libellés figés dans un montage ou une
préparation ne sont pas des identités concurrentes : ce sont des instantanés
historiques immuables. Ils doivent rester explicitement identifiés comme tels.

## Confusions de vocabulaire et de contrat

### Domaine et API

- `InstrumentRecord` désigne tout exemplaire physique, même un câble ou un
  accessoire.
- `register_metrology_instrument` crée l'identité du parc au lieu d'ouvrir un
  dossier métrologique.
- `/api/v1/metrology/instruments` sert à la fois de registre du parc et d'API
  métrologique.
- `MetrologyInstrument` côté TypeScript contient l'identité complète du parc.
- `PhysicalAssetMetrologyPanel` affiche et crée des exemplaires, puis leur
  métrologie, dans un composant unique.
- `asset_id` est présenté comme « Numéro d'inventaire ».
- `family`, `category_code` et les références de modèle sont acceptés
  directement depuis le client lors de l'enregistrement.
- le lien au modèle est facultatif lorsqu'une catégorie est fournie, ce qui
  contredit l'exigence de figer une version approuvée exacte.
- le numéro de série est obligatoire dans le core, SQLite, Rust, Python et le
  formulaire LAB CONSOLE.
- les anciens helpers Python et Qt écrivent encore directement l'identité dans
  `metrology.sqlite` lorsque l'agent n'est pas utilisé.
- `PlannedTestInstrumentAssignment` limite abusivement les matériels du
  laboratoire aux instruments.

### Interface normale

Les chaînes suivantes sont ambiguës, techniques, non accentuées ou en anglais
sur des écrans opérateur :

- `Équipements`, `Catalogue équipements`, `Matériels réels` ;
- `Fiche modèle équipement`, `Modèle d'équipement`, `Registre métrologique` et
  `Dossier métrologique` comme en-tête de l'exemplaire entier ;
- `Numéro d'inventaire` alors que la valeur affichée est l'ID interne ;
- `Part number` ;
- `Synthese`, `Categorie et formulaire`, `Caracteristiques`, `Revisions et
  audit`, `Diagnostic avance` ;
- `Valider`, `Soumettre`, `Approuver`, `Nouvelle revision` sans objet métier
  explicite ;
- `Functional role`, `Signal domains`, `Technology tags`, `Preset reference`,
  `Classification notes` ;
- `Model`, `Model revision`, `Checksum` dans le panneau de driver ;
- les références techniques `entity_id@revision_id`, les empreintes, IDs de
  source et IDs de révision visibles dans des formulaires de correction ;
- `Aggregate`, `asset`, `instrument` et noms de champs anglais encore présents
  dans certains états, diagnostics ou contrats de composants ;
- `Matériels réels du montage` dans Qt, qui ne distingue pas clairement les
  matériels du laboratoire de l'objet soumis à l'essai.

Les nouveaux libellés normaux seront `Catalogue des modèles`, `Modèle
constructeur`, `Parc matériel`, `Exemplaire du parc`, `Code inventaire`,
`Métrologie du parc`, `Matériels du laboratoire` et `Objet soumis à l'essai`.
Les IDs, empreintes et versions internes seront regroupés sous `Détails
techniques` ou `Diagnostic avancé`.

## Listes plates à remplacer

- le catalogue affiche un filtre d'arbre facultatif puis une liste plate de
  modèles; le même chemin de catégorie est même rendu deux fois par ligne ;
- le panneau physique effectue un simple `props.instruments.map` sans famille,
  catégorie, modèle ni emplacement hiérarchiques ;
- les drivers sont regroupés par modèle, mais affichent le `category_code` brut
  et ne suivent pas la hiérarchie métier ;
- le sélecteur des matériels en préparation est une liste d'options plate
  limitée au montage choisi ;
- le sélecteur Qt des matériels de montage est une liste plate issue des
  instruments métrologiques ;
- les résultats de recherche du catalogue et du parc ne garantissent pas un
  fil d'Ariane complet ;
- aucun mode de regroupement du parc par emplacement, état de service ou
  échéance métrologique n'existe.

La hiérarchie des catégories d'administration existe déjà. Elle ne doit pas
être confondue avec la navigation principale du catalogue, actuellement plate.

## Chargements tout-ou-rien

Les couplages suivants doivent être supprimés ou confinés :

- `EquipmentWorkspace.refresh` charge dans un seul `Promise.all` les modèles,
  drivers, providers, catégories, arbre, champs, instruments, conversions
  temporelles et réponses fréquentielles; un provider indisponible masque donc
  aussi le catalogue et le parc ;
- l'ouverture d'un modèle couple fiche, historique des versions et audit ;
- l'administration de catégorie couple formulaire effectif et règles directes ;
- `PhysicalAssetMetrologyPanel` couple caractérisations, corrections, file de
  revue et résolution; une panne de correction peut masquer le dossier utile ;
- `MeasurementEngineeringPanel` couple toutes les collections de définitions ;
- `ProjectWorkspace` couple fiche, revue de contrat, planning et audit ;
- l'assistant de préparation couple options disponibles et historique des
  préparations.

Chaque espace 0.22.0 devra conserver ses dernières données valides et afficher
l'erreur au plus près de la source défaillante. En particulier, une panne de
métrologie ne doit jamais retirer l'identité du parc et une panne de driver ne
doit jamais vider le catalogue.

## Workflows exigeant un exemplaire

Les montages et la préparation portent déjà des `asset_id`; un simple modèle
constructeur n'est pas volontairement proposé par LAB CONSOLE. Cette règle doit
devenir vérifiable par le domaine `fleet`, et non dépendre du fait qu'un ID se
trouve par hasard dans `metrology.instruments`.

Les faiblesses à corriger sont :

- l'API de montage reçoit la révision et l'empreinte du modèle depuis le client
  au lieu de les résoudre depuis l'exemplaire autoritatif ;
- l'enregistrement d'un « instrument » métrologique peut créer un exemplaire
  sans modèle approuvé exact ;
- les montages et préparations chargent l'identité dans la base métrologique ;
- la préparation fabrique le code inventaire depuis l'ID interne ;
- les sélecteurs n'expliquent pas systématiquement catégorie, lieu, état de
  service, disponibilité et statut métrologique ;
- les noms internes `instrument_assignment` survivent alors que le besoin est
  une affectation de matériel du laboratoire.

## Données 0.21.1 à préserver

La migration doit conserver et rendre lisibles après redémarrage :

1. chaque ligne `instruments`, avec son `asset_id`, fabricant, modèle, numéro de
   série, part number, famille, catégorie, lien exact au modèle, capacités,
   dates et révision ;
2. l'état de service et sa raison, ainsi que l'ancienne disponibilité nécessaire
   à une conversion déterministe ;
3. les exigences, périodes, avertissements et notes d'étalonnage ;
4. tous les `calibration_records` et `calibration_events` ;
5. tous les `instrument_documents` et leurs références de fichiers ;
6. toutes les `asset_characterization_events` ;
7. toutes les `asset_correction_assignments`, y compris leur cycle de revue ;
8. les événements d'audit métrologique, opérations et outbox existants ;
9. les bindings et instantanés déjà sérialisés dans les versions de montages ;
10. les instantanés déjà sérialisés dans les préparations planifiées ;
11. les couples lieu/label du planning et des montages, y compris les lieux
    historiques dont l'identité reste absente ;
12. les références exactes aux versions de modèles déjà approuvées.

Un instrument 0.21.1 sans lien complet vers un modèle approuvé ne doit pas être
perdu. Il sera importé comme exemplaire lisible avec un état de migration
explicite et non sélectionnable pour un nouveau montage tant que son modèle
n'aura pas été rapproché. Aucune référence de modèle ne sera inventée.

## Architecture cible 0.22.0

### Propriété des données

```text
equipment.sqlite
  Modèle constructeur (identité + versions)
  Exemplaire du parc (identité et cycle de vie autoritatifs)
  Lieu du laboratoire (identité et cycle de vie autoritatifs)

metrology.sqlite
  Dossier métrologique référencé par asset_id
  Étalonnages, certificats, caractérisations et corrections

station.sqlite / projects.sqlite
  Références d'asset_id et location_id
  Instantanés historiques lisibles et immuables
```

Le core introduira des types dédiés, sans dépendance HTTP ou SQLite :

- `PhysicalAsset`, `PhysicalAssetId`, `InventoryCode`, `OwnershipSource` ;
- `ServiceState` et `AvailabilityState`, avec transitions distinctes ;
- `PinnedEquipmentModel` contenant ID, version approuvée, empreinte et
  instantané lisible ;
- `LaboratoryLocation` et son état actif/archivé ;
- des erreurs structurées de validation et de transition.

Un exemplaire devra pinner une version `approved` existante. Une nouvelle
version du modèle ne le repointera jamais. Le fabricant, le modèle, la variante
et le chemin de catégorie conservés dans l'exemplaire seront des instantanés
historiques dérivés, non des champs libres concurrents.

### Persistance cible

Le domaine equipment possédera au minimum :

- `physical_assets` ;
- `physical_asset_audit_events` ;
- `physical_asset_operations` ;
- `laboratory_locations` ;
- `laboratory_location_audit_events` ;
- `laboratory_location_operations` ;
- une table de suivi de migration inter-domaines.

Les écritures de parc et de lieu utiliseront `BEGIN IMMEDIATE`, concurrence
optimiste, idempotence par `operation_id`, audit et outbox atomiques.

Le domaine métrologique possédera un dossier ne contenant que la politique
métrologique et la référence `asset_id`. Les tables de preuves conserveront
leur clé métier, mais ne dépendront plus d'une table d'identité physique locale.
L'existence de l'exemplaire sera validée par le service applicatif sous une
transaction SQLite attachant les bases nécessaires. SQLite ne pouvant pas
déclarer de clé étrangère entre deux fichiers, cette règle sera couverte par le
service, les contrôles d'intégrité et les tests de migration.

### Migration suivie

La migration sera composée de migrations SQL versionnées et d'un coordinateur
inter-domaines explicitement versionné dans le mécanisme `storage initialize` :

1. créer les nouvelles tables dans `equipment.sqlite` ;
2. créer le dossier métrologique sans identité dupliquée et reconstruire les
   tables dont la clé étrangère vise actuellement `instruments` ;
3. attacher `equipment.sqlite`, `metrology.sqlite` et `sync.sqlite` sous une
   même transaction `BEGIN IMMEDIATE` compatible avec la politique de journal
   du projet ;
4. importer chaque ligne 0.21.1 avec le même `asset_id` et un code inventaire
   déterministe non conflictuel ;
5. conserver les références exactes de modèle lorsqu'elles sont complètes ;
6. créer un dossier métrologique par ancien instrument ;
7. enregistrer audit et outbox de migration avec des `operation_id`
   déterministes ;
8. vérifier les nombres et références avant commit ;
9. inscrire la version dans le registre de migration inter-domaines ;
10. rendre toute nouvelle exécution idempotente.

L'ancienne table d'identité pourra être conservée sous un nom d'archive en
lecture de migration, mais aucun runtime 0.22.0 ne pourra la modifier ni la
considérer comme source de vérité. Il n'y aura ni dual-write ni seconde copie
modifiable.

### APIs cibles

Les nouveaux workflows utiliseront notamment :

```text
GET    /api/v1/fleet/assets
POST   /api/v1/fleet/assets
GET    /api/v1/fleet/assets/{asset_id}
PUT    /api/v1/fleet/assets/{asset_id}
POST   /api/v1/fleet/assets/{asset_id}/transitions/service-state
POST   /api/v1/fleet/assets/{asset_id}/transitions/availability
GET    /api/v1/fleet/assets/{asset_id}/audit-events

GET    /api/v1/laboratory-locations
POST   /api/v1/laboratory-locations
PUT    /api/v1/laboratory-locations/{location_id}
POST   /api/v1/laboratory-locations/{location_id}/archive
GET    /api/v1/laboratory-locations/{location_id}/audit-events

GET    /api/v1/metrology/assets/{asset_id}
PUT    /api/v1/metrology/assets/{asset_id}/dossier
```

Les routes de preuves métrologiques resteront rattachées à `asset_id`. Une
adaptation temporaire en lecture de `/api/v1/metrology/instruments` est admise
pour les clients 0.21.1, mais la création d'identité et les transitions de parc
ne passeront plus par cette API. Sa suppression sera documentée.

### Lieux et instantanés

Les nouveaux montages, créneaux et exemplaires sélectionneront un
`location_id` actif. Le serveur résoudra le libellé; le client ne pourra plus
imposer un couple ID/libellé incohérent. Renommer un lieu conservera son ID et
les anciennes versions garderont leur libellé instantané. Un lieu archivé
restera lisible mais sera refusé pour toute nouvelle affectation.

### Navigation et chargements

LAB CONSOLE séparera les espaces suivants sous `Ressources techniques` :

1. `Catalogue des modèles` ;
2. `Parc matériel` ;
3. `Métrologie du parc` ;
4. `Montages de mesure` ;
5. `Drivers et pilotage` ;
6. `Signaux et corrections` ;
7. `Lieux du laboratoire`.

Le catalogue utilisera l'arbre famille, sous-catégorie, fabricant, modèle. Le
parc utilisera famille, sous-catégorie, modèle constructeur, exemplaires, avec
des regroupements alternatifs par lieu, état de service et échéance
métrologique. Les panneaux auront chacun leur propre état de chargement,
d'erreur et de rafraîchissement.

## Invariants à vérifier pendant l'implémentation

- un modèle refuse tout champ d'identité ou de cycle de vie réservé au parc ;
- un exemplaire référence exactement une version approuvée et son empreinte ;
- le code inventaire est unique et le numéro de série optionnel ;
- l'ID interne n'est jamais demandé à l'opérateur ;
- état de service et disponibilité évoluent indépendamment et sous concurrence
  optimiste ;
- un lieu archivé ne peut plus être affecté ;
- la métrologie ne peut écrire ni fabricant, ni modèle, ni série, ni code
  inventaire, ni lieu, ni cycle de vie ;
- un modèle générique est refusé partout où un exemplaire est requis ;
- les montages et préparations résolvent l'identité dans le parc ;
- un refus n'écrit ni état partiel, ni audit, ni outbox ;
- l'import 0.21.1 est idempotent et conserve toutes les preuves ;
- un redémarrage conserve le parc, les lieux et les liens métrologiques ;
- aucune panne secondaire ne masque une identité déjà chargée.

## Risques identifiés

1. **Migration inter-bases.** Les migrations actuelles s'appliquent fichier par
   fichier. Le transfert d'identité exige un coordinateur suivi et atomique,
   pas un script de démarrage informel.
2. **Clés étrangères métrologiques.** Plusieurs tables référencent directement
   `instruments`; elles doivent être reconstruites sans perdre leurs lignes ni
   leurs index.
3. **Instruments non liés au catalogue.** Ils doivent rester lisibles mais ne
   peuvent pas recevoir une fausse version approuvée.
4. **Clients directs Python/Qt.** Les chemins d'écriture SQLite contournant
   l'agent créeraient un dual-write; ils doivent être remplacés ou explicitement
   rendus lecture seule.
5. **Instantanés JSON historiques.** Leur forme ne doit pas être réécrite; les
   lecteurs doivent accepter les anciennes versions et produire la nouvelle
   forme seulement pour les nouveaux enregistrements.
6. **États historiques.** `availability=reserved` et
   `availability=out_of_service` ont des sens mélangés; leur conversion doit
   être déterministe et tracée.
7. **Surface UI monolithique.** La séparation doit préserver les workflows de
   modèle, correction et driver existants tout en leur donnant des chargements
   indépendants.
8. **Preuves visuelles.** Les captures 0.18.0 à 0.21.1 sont immuables; seule la
   commande explicite de rafraîchissement 0.22.0 pourra écrire les nouvelles.

## Décision de passage à l'implémentation

L'architecture cible est retenue : `equipment.sqlite` devient la source de
vérité unique du parc et des lieux; `metrology.sqlite` ne conserve que les
dossiers et preuves rattachés par `asset_id`; station et projets conservent des
références et instantanés. Aucun dual-write d'identité physique n'est admis.

L'implémentation peut commencer après commit et publication de ce document et
de la mise à jour du journal de reprise.
