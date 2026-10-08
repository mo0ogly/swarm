"""Image de couverture LinkedIn (1920x1080) : même incident, avec et sans moteur. Scénario illustratif."""
from pathlib import Path

INK, MUTED, LINE = "#18394f", "#4a5a68", "#496781"
RED, RED_BG = "#a33a2f", "#fbe3e1"
GREEN, GREEN_BG = "#2f6b43", "#e4f2e8"
BLUE_BG, AMBER_BG = "#dceef8", "#fff0d1"


def text(x, y, s, size=26, color=INK, weight=400, anchor="middle"):
    return f'<text x="{x}" y="{y}" font-size="{size}" fill="{color}" font-weight="{weight}" text-anchor="{anchor}">{s}</text>'


def box(cx, y, w, h, fill, stroke, label, size=28, weight=600, color=INK):
    return (f'<rect x="{cx - w / 2}" y="{y}" width="{w}" height="{h}" rx="10" fill="{fill}" stroke="{stroke}" stroke-width="2"/>'
            + text(cx, y + h / 2 + size / 3, label, size, color, weight))


def arrow(x1, x2, y, dashed=False, marker="arr", color=LINE):
    dash = ' stroke-dasharray="12 9"' if dashed else ""
    return f'<line x1="{x1}" y1="{y}" x2="{x2}" y2="{y}" stroke="{color}" stroke-width="3"{dash} marker-end="url(#{marker})"/>'


def panel(ox, safe):
    a, b = ox + 200, ox + 650          # lignes de vie : règlement, banque
    accent, accent_bg = (GREEN, GREEN_BG) if safe else (RED, RED_BG)
    head = "Moteur déterministe + clé métier" if safe else "Sans contrôle pendant l’exécution"
    sub = "Lot validé · clé S01 + F-12 reconnue par la banque" if safe else "Chaque essai est une nouvelle demande"
    out = [f'<rect x="{ox}" y="230" width="850" height="740" rx="18" fill="#ffffff" stroke="{accent}" stroke-width="3"/>',
           f'<rect x="{ox}" y="230" width="850" height="92" rx="18" fill="{accent_bg}"/>',
           f'<rect x="{ox}" y="300" width="850" height="22" fill="{accent_bg}"/>',
           text(ox + 425, 275, head, 34, accent, 700), text(ox + 425, 309, sub, 24, MUTED),
           box(a, 350, 230, 72, BLUE_BG if safe else AMBER_BG, LINE, "Règlement"),
           box(b, 350, 230, 72, GREEN_BG, "#427553", "Banque"),
           f'<line x1="{a}" y1="422" x2="{a}" y2="800" stroke="#c9d3db" stroke-width="3"/>',
           f'<line x1="{b}" y1="422" x2="{b}" y2="800" stroke="#c9d3db" stroke-width="3"/>',
           text((a + b) / 2, 468, "1 · Payer F-12 · 4 200 €", 25), arrow(a + 6, b - 8, 482),
           text((a + b) / 2, 548, "2 · Réponse perdue", 25, MUTED), arrow(b - 6, a + 30, 562, dashed=True, marker="none"),
           f'<path d="M{a + 14} 548 l26 28 M{a + 40} 548 l-26 28" stroke="{RED}" stroke-width="4"/>',
           text((a + b) / 2, 628, "3 · Nouvel essai" + (" · même clé" if safe else ""), 25), arrow(a + 6, b - 8, 642)]
    if safe:
        out += [text((a + b) / 2, 708, "4 · Clé connue : déjà exécuté", 25, GREEN, 600),
                arrow(b - 6, a + 8, 722, dashed=True, marker="arrg", color=GREEN)]
    else:
        out += [text((a + b) / 2, 708, "4 · Clé absente : nouveau virement", 25, RED, 600),
                arrow(a + 6, b - 8, 722, dashed=True, marker="arrr", color=RED)]
    result = "1 virement · 4 200 € débités" if safe else "2 virements · 8 400 € débités"
    out += [f'<rect x="{ox + 70}" y="830" width="710" height="100" rx="14" fill="{accent_bg}" stroke="{accent}" stroke-width="2"/>',
            text(ox + 425, 893, result, 38, accent, 700)]
    return "".join(out)


svg = f'''<svg xmlns="http://www.w3.org/2000/svg" width="1920" height="1080" viewBox="0 0 1920 1080" font-family="Liberation Sans, Arial, sans-serif">
<defs>
 <marker id="arr" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="7" markerHeight="7" orient="auto"><path d="M0 0L10 5L0 10z" fill="{LINE}"/></marker>
 <marker id="arrr" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="7" markerHeight="7" orient="auto"><path d="M0 0L10 5L0 10z" fill="{RED}"/></marker>
 <marker id="arrg" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="7" markerHeight="7" orient="auto"><path d="M0 0L10 5L0 10z" fill="{GREEN}"/></marker>
</defs>
<rect width="1920" height="1080" fill="#f6f8fa"/>
{text(960, 118, "Une réponse perdue. Deux virements ?", 66, INK, 700)}
{text(960, 178, "Le même incident de paiement, avec et sans moteur déterministe pour piloter les agents IA", 30, MUTED)}
{panel(80, False)}
{panel(990, True)}
{text(80, 1035, "Fabrice Pizzi · Agents IA et processus critiques", 24, MUTED, 400, "start")}
{text(1840, 1035, "Scénario illustratif · données fictives", 24, MUTED, 400, "end")}
</svg>'''
Path(__file__).with_name("cover.svg").write_text(svg)
print("cover.svg écrit")
