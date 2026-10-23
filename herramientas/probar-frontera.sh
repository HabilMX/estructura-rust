#!/usr/bin/env bash
# La frontera que sabe fallar: comprueba que lo que la biblioteca no declara
# `pub` NO se alcanza desde tests/.
#
# Qué hace, en una COPIA temporal del proyecto (tu carpeta no se toca):
#
#   Siembra tests/frontera.rs, una prueba de integración que llama a
#   revisor::reporte::url_publica. Esa función es privada a propósito: es un
#   detalle de cómo se arma el reporte. Cargo compila cada archivo de tests/
#   como un crate aparte, así que esa prueba solo ve lo que la biblioteca
#   declara `pub`. Luego compila la MISMA prueba dos veces, sobre la MISMA
#   copia. Entre las dos solo cambia una palabra de src/reporte.rs: `pub`.
#
#   1. CONTROL. Con «pub fn url_publica», la prueba tiene que compilar y pasar.
#      Si no, el experimento está mal montado (la función cambió de nombre o de
#      lugar, la prueba sembrada tiene un error, no hay red para bajar las
#      dependencias) y el paso 2 no probaría nada: se para en rojo.
#   2. VIOLACIÓN. Con src/reporte.rs tal como está en tu proyecto, la misma
#      prueba tiene que NO compilar, y por la razón exacta:
#      «error[E0603]: function `url_publica` is private», en la prueba y no en
#      la biblioteca. Si compila, la frontera no existe. Si falla por otra
#      cosa, tampoco demuestra nada. En los dos casos sale en rojo.
#
# Uso:  bash herramientas/probar-frontera.sh   → sale 0 si la frontera se sostiene
#
# Por qué así: un experimento que solo espera un rojo se pone «verde» también
# cuando falla por la razón equivocada. Lo impiden tres cosas: el control
# demuestra que la prueba compila y pasa cuando la función es pública; la
# violación exige el código de error exacto y que falle la prueba, no la
# biblioteca; y el conteo del control se lee del informe de la prueba
# («test result: ok. 1 passed»), no del código de salida de cargo.
set -euo pipefail

raiz="$(cd "$(dirname "$0")/.." && pwd)"
archivo="src/reporte.rs"
funcion="url_publica"
camino="revisor::reporte::$funcion"
esperado="error[E0603]: function \`$funcion\` is private"

tmp="$(mktemp -d)"
# Al salir se borra la copia, y el código de salida se conserva. Con el bash
# 3.2 que trae macOS, si el guion se corta por una variable sin definir
# (set -u), la trampa recibe un 0 en «$?»: sin la bandera «terminado», un
# guion cortado a la mitad saldría 0, un falso verde (medido). Por eso, si no
# llegó al final, sale 1 aunque «$?» diga 0.
salir() {
  local codigo=$?
  rm -rf "$tmp"
  if [ "$codigo" -eq 0 ] && [ "${terminado:-}" != 1 ]; then
    echo "❌ el guion se cortó antes de terminar" >&2
    codigo=1
  fi
  exit "$codigo"
}
trap salir EXIT
copia="$tmp/copia"
mkdir -p "$copia"

# La copia lleva todo menos lo compilado y la historia de git. La «m» de
# «tar -xmf» les pone a los archivos copiados la hora de ahora: cargo decide qué
# recompilar comparando fechas, y una copia con las fechas viejas podría reusar
# lo que se compiló en una corrida anterior con OTRO código.
(cd "$raiz" && tar --exclude=./target --exclude=./.git -cf - .) | (cd "$copia" && tar -xmf -)

# Lo compilado de la frontera va aparte de tu target/, para no mezclarse con
# tus corridas; se conserva entre corridas para no recompilar todo cada vez.
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$raiz/target}/frontera"
export CARGO_TERM_COLOR=never

cat >"$copia/tests/frontera.rs" <<EOF
// Sembrada por herramientas/probar-frontera.sh: llama a $funcion desde
// fuera de la biblioteca.
#[test]
fn ${funcion}_desde_tests() {
    assert_eq!(
        $camino("https://usuario:clave@pagos.ejemplo.com/salud?token=x"),
        "https://pagos.ejemplo.com/salud"
    );
}
EOF

# Compila y corre SOLO la prueba sembrada. Deja la salida en $1.
correr() { (cd "$copia" && cargo test --locked --test frontera >"$1" 2>&1); }

# ${funcion} va con llaves: sin ellas, el bash 3.2 de macOS lee el primer byte
# de la comilla de cierre como parte del nombre de la variable.
echo "1/2 control: con «pub fn ${funcion}», una prueba de tests/ la llama"
# Cualquier visibilidad que tenga (ninguna, pub(crate), pub) pasa a ser pub.
sed -E "s/^(pub(\([a-z]+\))? )?fn $funcion\(/pub fn $funcion(/" "$raiz/$archivo" >"$copia/$archivo"
correr "$tmp/control.log" || true
resumen="$(grep -m1 '^test result:' "$tmp/control.log" || true)"
pasaron="$(printf '%s\n' "$resumen" | sed -nE 's/.* ([0-9]+) passed;.*/\1/p')"
fallaron="$(printf '%s\n' "$resumen" | sed -nE 's/.* ([0-9]+) failed;.*/\1/p')"
if [ "${pasaron:-0}" != "1" ] || [ "${fallaron:-1}" != "0" ]; then
  echo "❌ el control NO pasó; el experimento está mal montado:"
  echo "     ${resumen:-(no hubo informe de pruebas: no compiló)}"
  sed 's/^/     /' "$tmp/control.log" | tail -n 25
  exit 1
fi
echo "   ✅ compiló y pasó (1 de 1): la prueba sembrada está bien escrita"

echo "2/2 violación: la misma prueba, con $archivo tal como está"
cp "$raiz/$archivo" "$copia/$archivo"
if correr "$tmp/violacion.log"; then
  echo "❌ COMPILÓ: una prueba de tests/ pudo llamar a $camino. La frontera no existe."
  exit 1
fi
if ! grep -F -q "$esperado" "$tmp/violacion.log" ||
  ! grep -F -q 'error: could not compile `revisor` (test "frontera") due to 1 previous error' "$tmp/violacion.log"; then
  echo "❌ falló, pero NO por la privacidad de $funcion. Esto no demuestra la frontera:"
  sed 's/^/     /' "$tmp/violacion.log" | tail -n 25
  exit 1
fi
grep -F -A1 "$esperado" "$tmp/violacion.log" | sed 's/^/     /'
echo "   ✅ el compilador la rechazó: lo que no es pub no se ve desde tests/"
echo "✅ frontera: se sostiene"
terminado=1
