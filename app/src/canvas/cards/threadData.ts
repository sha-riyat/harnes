// The thread of lot L1, as agents really write it: a long brief, a short question,
// a long report. Synthetic, like the rest of cardData. `tick` is when a message
// arrives in the replay (0 = already there); `min` is minutes since the first one.

import type { Role } from "../../ui/kit/roles";

export type ThreadMsg = { id: number; kind: "msg" | "event"; role?: Role; from: string; to?: string; at: string; min: number; tick: number; text: string };

export const lotThread = {
  lot: "L1",
  title: "Icônes typographiques sur les touches",
  status: "open" as "open" | "closed",
  messages: [
    {
      id: 1, kind: "msg", role: "planner", from: "Planner", to: "Worker L1", at: "14:21", min: 0, tick: 0,
      text: 'Lot L1 (feature). Fichiers : index.html, styles.css, tests/glyphs.e2e.yaml\nDans index.html, remplacer uniquement le contenu visible de quatre boutons, sans toucher a un seul attribut : (1) priorite absolue, la touche data-key="back" passe du libelle <- au caractere de retour arriere U+232B, ecrit en entite numerique &#x232B; ; (2) data-key="/" affiche ÷ (U+00F7, &#xF7;) ; (3) data-key="*" affiche × (U+00D7, &#xD7;) ; (4) data-key="-" affiche − (U+2212 MINUS SIGN, &#x2212;), pas le trait d\'union ASCII. Laisser strictement inchanges +, =, %, &radic;, C, les chiffres et le point.\nINTERDIT : modifier un attribut data-key (back, /, *, -, sqrt, %, C, =, chiffres, point), modifier app.js, le map clavier ou les raccourcis acceptes (Backspace, %, r, et / * - + du clavier). Le fichier est en charset UTF-8 : si tu ecris les caracteres litteraux, verifier que l\'encodage du fichier reste UTF-8 sans BOM ; en cas de doute, preferer les entites numeriques.\nVerifier les aria-label : back = retour arriere, / = division, * = multiplication, - = soustraction doivent rester en place et en francais ; ajouter un aria-label a tout bouton qui porterait desormais une icone sans libelle accessible (a priori aucun, les quatre en ont deja un). Ensuite controler le rendu dans le vrai navigateur via harnes-e2e : e2e_open puis e2e_screenshot, et enregistrer la capture sous .harnes/runs/1792246715/keys-before-after ou un chemin equivalent que tu citeras dans ton rapport. Verifier sur la capture : aucun carre de police manquante (tofu), chaque glyphe centre dans sa touche, et une taille optique comparable aux chiffres voisins. ⌫, ÷, × et − n\'ont ni la meme largeur ni la meme hauteur que les chiffres : si c\'est le cas, ajuster uniquement font-size et line-height dans styles.css, sans toucher aux data-key.',
    },
    { id: 2, kind: "msg", role: "worker", from: "Worker L1", to: "Planner", at: "14:26", min: 5, tick: 0, text: "Question : le bouton back porte déjà un aria-label en français, je le garde tel quel. Je n'ajoute donc aucun aria-label. Confirmé ?" },
    { id: 3, kind: "msg", role: "planner", from: "Planner", to: "Worker L1", at: "14:27", min: 6, tick: 0, text: "Confirmé. Aucun aria-label à ajouter." },
    {
      id: 4, kind: "msg", role: "worker", from: "Worker L1", to: "Planner", at: "14:52", min: 31, tick: 9,
      text: "Fait. Fichiers : index.html, styles.css, tests/glyphs.e2e.yaml\nLes quatre boutons affichent maintenant ⌫, ÷, × et −, écrits en entités numériques (&#x232B; &#xF7; &#xD7; &#x2212;). Aucun attribut data-key n'a changé, app.js non plus, et le fichier reste en UTF-8 sans BOM.\nVérifier : le rendu dans le vrai navigateur est bon. Aucun carré de police manquante. ÷ et × étaient un peu petits : j'ai porté font-size à 1.15em et line-height à 1 sur ces deux touches dans styles.css. Capture : .harnes/runs/1792246715/keys-before-after.png\nTests : tests/glyphs.e2e.yaml passe (4 sur 4).\n```\n$ npm test\n  glyphs.e2e.yaml\n    ✓ back shows the backspace glyph\n    ✓ division shows ÷\n    ✓ multiplication shows ×\n    ✓ subtraction shows the true minus\n```",
    },
    { id: 5, kind: "event", role: "teacher", from: "Teacher", at: "14:53", min: 32, tick: 15, text: "Teacher closed the thread: lot terminé" },
  ] as ThreadMsg[],
};
