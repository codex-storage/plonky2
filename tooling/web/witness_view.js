
const the_supported_gates = 
  [ "UnknownGate"
  , "ArithmeticGate"
  , "ArithmeticExtensionGate"
  , "BaseSumGate"
  , "ConstantGate"
  , "CosetInterpolationGate"
  , "ExponentiationGate"
  , "LookupGate"
  , "LookupTableGate"
  , "MultiplicationExtensionGate"
  , "NoopGate"
  , "PoseidonGate"
  , "PoseidonMdsGate"
  , "PublicInputGate"
  , "RandomAccessGate"
  , "ReducingGate"
  , "ReducingExtensionGate"
  ];

const the_gate_colors = 
  [ "#800000"     // UnknownGate                 -> dark red
  , "#c0c0c0"     // ArithmeticGate              -> light grey 
  , "#808080"     // ArithmeticExtensionGate     -> medium grey
  , "#60c0c0"     // BaseSumGate                 -> medium cyan
  , "#f0f080"     // ConstantGate                -> yellow
  , "#b0b0f0"     // CosetInterpolationGate      -> light blue
  , "#80f080"     // ExponentiationGate          -> cyan
  , "#80f080"     // LookupGate                  -> light green
  , "#60c060"     // LookupTableGate             -> medium green
  , "#8080c0"     // MultiplicationExtensionGate -> dark blue
  , "#ffffff"     // NoopGate                    -> white
  , "#f07070"     // PoseidonGate                -> light red
  , "#c06060"     // PoseidonMdsGate             -> medium red
  , "#a0ffa0"     // PublicInputGate             -> ligth green
  , "#8080f0"     // RandomAccessGate            -> blue
  , "#f080f0"     // ReducingGate                -> light purple
  , "#c060c0"     // ReducingExtensionGate       -> medium purple
  ];

function findGateIndex(gate) {
  let k = the_supported_gates.indexOf(gate);
  return ((k<0) ? 0 : k);
} 

function add_table_row(table, idx, text, color) {
  let tr = document.createElement("tr");
  tr.style.background = (color)?color:"#fffff";

  let td1 = document.createElement("td"); td1.innerHTML = idx.toString();
  let td2 = document.createElement("td"); td2.innerHTML = text;

  tr.appendChild(td1);
  tr.appendChild(td2);
  table.appendChild(tr);
}

/*
function test_fill_gates() {
  let el = document.getElementById("gates");
  for(let i=0; i<the_supported_gates.length; i++) {
    add_table_row(el, the_supported_gates[i], the_gate_colors[i]);
  }
}
*/

var gates;
var matrix;
var selectors;
var ngates;
var nrows;
var ncolumns;
var ncells;

function cell_hover(row,column) {
  let el_row  = document.getElementById("cell-row"   );
  let el_col  = document.getElementById("cell-column");
  let el_val  = document.getElementById("cell-value" );
  let el_gate = document.getElementById("cell-gate"  );
  el_row.innerHTML  = row.toString();
  el_col.innerHTML  = column.toString();
  el_val.innerHTML  = matrix[column][row].toString();
  el_gate.innerHTML = gates[selectors[row]];
}

function initialize_from_witness(fname,json) {
  //console.log(json);

  let el_fname = document.getElementById("fname");
  el_fname.value = fname;

  // these are global variables!
  gates     = json.gates;
  matrix    = json.matrix;
  selectors = json.selector_vector;
  ngates    = gates.length;
  nrows     = selectors.length;
  ncolumns  = matrix.length
  ncells    = ncolumns * nrows;

  let gates_base   = [];
  let gate_colors  = [];

  for(let i=0; i<gates.length; i++) {
    let full = gates[i];
    let base = full.split(":")[0];
    gates_base[i] = base;

    k = findGateIndex(base);
    gate_colors[i]  = the_gate_colors[k];

  }

  let el_table = document.getElementById("gates");
  for(let i=0; i<ngates; i++) {
    add_table_row(el_table, i, gates[i], gate_colors[i]);
  }

  let el_svg = document.getElementById("matrix");
  el_svg.setAttribute('width'  , 10*ncolumns);
  el_svg.setAttribute('height' , 10*nrows   );

  let empty_counter = 0;
  for(let j=0; j<ncolumns; j++) {
    let column = matrix[j];
    for(let i=0; i<nrows; i++) {
      let sel = selectors[i];
      let col = gate_colors[sel];
      let val = column[i];

      if (val == 0) { 
        col = "#000";
        empty_counter ++;
      }

      // <rect width="10" height="10" x="20" y="20" style="fill:rgb(128,128,255);stroke-width:1;stroke:black" />
      let cell = document.createElementNS("http://www.w3.org/2000/svg", "rect"); 
      cell.setAttribute('width' ,10);
      cell.setAttribute('height',10);
      cell.setAttribute('x',10*j);
      cell.setAttribute('y',10*i);
      cell.setAttribute('style',"stroke-width:1;stroke:black;");
      cell.setAttribute('fill',col);
      let hover_call = "cell_hover(" + i.toString() + "," + j.toString() + ")";
      cell.setAttribute('onmouseover', hover_call );
      el_svg.appendChild(cell);
    }
  }

  el_ratio = document.getElementById("nonempty-ratio");
  let ratio = Math.round( 100 * (ncells - empty_counter) / ncells);
  el_ratio.value = ratio.toString() + "%";
}

function handle_error(res) {
  if (!res.ok) {
    throw new Error(`HTTP error! Status: ${res.status}`);
  }
  return res.json();
}

function load_witness(fname) {
  fetch(fname).then( err => handle_error(err) ).then( data => initialize_from_witness(fname,data) ).catch();
}

