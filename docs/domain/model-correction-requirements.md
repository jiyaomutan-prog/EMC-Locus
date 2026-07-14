# Exigences de correction d'un modele

## Objet

Une exigence de correction appartient a une revision de modele d'equipement. Elle
dit **ce qui doit etre corrige**, sur quel chemin du signal et selon quelle
politique. Elle ne contient pas implicitement la valeur mesuree d'un materiel
reel.

`CorrectionRequirementDefinition` est type dans `emc-locus-core` et contient :

- une identite stable dans le chemin (`requirement_id`, `signal_path_id`) ;
- un nom et un but physique lisibles ;
- le type `raw_signal_conversion` ou `frequency_dependent_correction` ;
- l'operation a appliquer et les grandeurs/unites attendues ;
- le caractere obligatoire pour l'utilisation ;
- la politique de valeur propre au materiel ;
- des conditions d'application telles que la polarisation ;
- eventuellement une reference epinglee vers une valeur nominale du modele.

## Politiques

| Politique interne | Sens metier |
| --- | --- |
| `asset_required` | Chaque numero de serie doit avoir sa propre correction approuvee. |
| `asset_preferred` | La valeur propre au materiel est prioritaire ; un repli peut etre autorise selon le contexte. |
| `model_value_allowed` | Une valeur nominale approuvee peut etre utilisee hors contexte accredite. |
| `model_value_only` | La correction est volontairement commune au modele. |

Une reference nominale est epinglee par identite, revision et checksum. Sa
qualite est explicite : `manufacturer_nominal`, `manufacturer_typical`,
`model_characterization` ou `simulation_only`. La valeur nominale reste une
donnee de modele ; elle ne devient jamais une mesure attribuee a un numero de
serie.

## Exemples

- cable RF : pertes selon la frequence, valeur propre obligatoire ;
- antenne de reception : facteur d'antenne horizontal ou vertical, conditionne
  par la polarisation et propre a l'antenne ;
- amplificateur RF : gain d'amplitude et phase facultative selon la frequence ;
- sonde de courant : conversion nominale du modele, sensibilite etalonnee du
  materiel prioritaire ;
- accelerometre IEPE : sensibilite nominale `100 mV/g`, sensibilite etalonnee du
  numero de serie prioritaire.

## Invariants

Le core refuse les identifiants invalides, les exigences dupliquees, les unites
vides, les chemins incoherents, les references nominales incompletes et le
melange, dans un meme chemin, des anciennes transformations et des nouvelles
exigences. Le service agent verifie en plus que chaque valeur nominale pointe
vers une revision approuvee ou remplacee dont le checksum est exact.

Les anciennes revisions restent lisibles pour permettre une nouvelle revision
controlee. LAB CONSOLE propose une migration explicite de leurs references de
transformation vers les exigences ; le resoluteur 0.18.0 ne deduit jamais une
valeur propre au materiel a partir de ces anciennes references.
