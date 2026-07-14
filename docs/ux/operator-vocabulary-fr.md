# Vocabulaire operateur francais

## Libelles approuves

- Entrees, sorties et corrections attendues
- Conversion du signal brut
- Correction selon la frequence
- Correction attendue
- Correction du materiel
- Valeur nominale du modele
- Valeur propre a ce materiel
- Pertes du cable
- Facteur d'antenne
- Gain de l'amplificateur
- Sensibilite de la sonde
- Source de la correction
- Certificat ou caracterisation
- Valide a partir du
- Valide jusqu'au
- Correction manquante
- En attente de revue
- Active pour ce materiel
- Correction expiree
- Correction remplacee
- Pret pour un essai
- Non pret pour un essai

## Termes interdits dans le parcours normal

`Engineering curve`, `Courbe d'ingenierie`, `Scaling profile`, `Profil de
scaling`, `Transformation`, `Transformation slot`, `Artifact`, `Assignment`,
`Entity`, `Aggregate`, `Resolver`, `Signal representation`, `Definition
checksum`, `Revision identifier`, `Asset correction assignment` et les codes
`snake_case` ne sont pas des libelles operateur.

## Exemples concrets

- un cable fournit ses pertes en dB selon la frequence ;
- une antenne fournit son facteur d'antenne par polarisation ;
- un amplificateur fournit son gain, et eventuellement sa phase, selon la
  frequence ;
- une sonde de courant fournit sa sensibilite en mV/A ;
- un accelerometre IEPE fournit sa sensibilite en mV/g.

Les identifiants, revisions, checksums, versions de schema, traces de resolution
et JSON brut sont reserves au volet replie `Diagnostic technique`, accompagne
de la mention `Informations destinees au support et aux developpeurs.`
