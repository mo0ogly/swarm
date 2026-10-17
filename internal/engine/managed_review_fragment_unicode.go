//go:build linux

package engine

import (
	"strings"
	"unicode/utf8"
)

const managedFragmentInspectionContractVersion = 1

func supportedFragmentInspectionContract(version int) bool {
	return version == 0 || version == managedFragmentInspectionContractVersion
}

func fragmentNeedLength(need string, version int) (minimum, maximum int) {
	if version == 0 {
		return len(strings.TrimSpace(need)), len(need)
	}
	return utf8.RuneCountInString(strings.TrimSpace(need)), utf8.RuneCountInString(need)
}

const managedFragmentInspectionContractInstructions = `
CONTRAT D'INSPECTION1 : n contient des demandes distinctes de 3 à 240 caractères Unicode chacune, comme minLength/maxLength du schéma ; les octets UTF-8 ne sont pas des caractères. La borne totale de réponse reste applicable. Rédiger des besoins précis et complets sous ces bornes.
TRANSPORT DU TEXTE : la pièce context conserve les métadonnées canoniques avec diff vidé et sources omises parce que ces originaux sont des pièces distinctes de l'inventaire. Ce diff vide ne signifie pas delta Git vide. L'inventaire identifie les sections du delta complet ; chaque section originale est dans un paquet, et le décideur sélectionne celles nécessaires. Ne pas inférer une contradiction du candidat depuis cette seule omission de transport ; une section nécessaire absente de ce paquet donne une demande précise pour la sélection finale.
`
