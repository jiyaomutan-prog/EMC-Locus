# Equipment Taxonomy

Release `0.13.1` introduces the user-facing equipment taxonomy used by LAB
CONSOLE. The taxonomy is a business repository layer above the internal
equipment class, functional role, signal-domain and technology-tag model.

## Root Categories

Fresh storage initialization creates seven system-defined roots:

- Sources d'énergie
- Sources de signaux
- Équipements radiofréquences
- Capteurs / transducteurs
- Actionneurs / Émetteurs
- Instruments de mesure / numériseurs
- Systèmes de traitement et de contrôle

Root category ids are stable internal codes. Normal users see labels, not raw
codes. Root categories cannot be archived, deleted, or moved.

## Subcategories

Subcategories can be created, edited, moved and archived by Repository
Administration when they are not in use. Default subcategories include RF
cables, attenuators, couplers, amplifiers, filters, loads, receiving antennas,
field probes, current probes, accelerometers, microphones, EMC receivers,
spectrum analyzers, oscilloscopes, RF power meters, multimeters and DAQ.

Every model belongs to a root either directly or through a subcategory. The
searchable model summary stores both `category_code` and `root_category_id`.

## Technical Mapping

The taxonomy does not replace the technical core. Creating a model from a
category template derives conservative internal defaults for equipment class,
functional role, signal domains, technology tags, ports and interfaces. Those
technical details remain visible in Advanced / Diagnostics.
