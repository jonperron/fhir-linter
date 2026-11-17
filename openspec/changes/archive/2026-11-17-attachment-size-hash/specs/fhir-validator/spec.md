## ADDED Requirements

### Requirement: Validation des pièces jointes — size et hash (ATTACHMENT_001, ATTACHMENT_002)

Pour tout objet Attachment, à toute profondeur, lorsque `data` est présent et se décode en base64 (espaces ASCII ignorés, selon base64Binary), le système SHALL le comparer à `size` et `hash`. Il SHALL émettre `ATTACHMENT_001` (erreur, au `path` de l'Attachment) lorsque `size` (chaîne integer64 ou nombre JSON) diffère du nombre d'octets décodés. Il SHALL émettre `ATTACHMENT_002` (erreur, au `path` de l'Attachment) lorsque `hash`, décodé, diffère du SHA-1 des octets décodés ; le message SHALL donner les hashs déclaré et réel en base64. Aucun diagnostic SHALL être émis si `data` est absent (le contenu peut être à `url`), si `data` ou `hash` ne se décode pas, ou si `size` n'est pas un entier : les contrôles de format les signalent.

#### Scenario: size en désaccord

- **WHEN** un Attachment a un `data` de 5 octets décodés et `size` = `10`
- **THEN** un Diagnostic d'erreur `ATTACHMENT_001` SHALL être émis au `path` de l'Attachment

#### Scenario: hash en désaccord

- **WHEN** le `hash` décodé d'un Attachment diffère du SHA-1 des octets de `data`
- **THEN** un Diagnostic d'erreur `ATTACHMENT_002` SHALL être émis, son message donnant les hashs déclaré et réel

#### Scenario: rien à comparer

- **WHEN** `data` est absent, ou `data`/`hash` ne se décode pas, ou `size` n'est pas un entier
- **THEN** aucun Diagnostic `ATTACHMENT_001` ni `ATTACHMENT_002` SHALL être émis
