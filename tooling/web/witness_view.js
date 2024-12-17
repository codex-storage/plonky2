
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

function div(a,b) { return Math.floor(a/b); }

function role_Unknown(i,params) { return "n/a"; }

function role_ArithmeticGate(i,params) { 
  let numops = params[0];
  if (i>=4*numops) return "unused"; 
  let k = div(i,4);
  switch(i%4) { 
    case 0: return "x" + k;
    case 1: return "y" + k;
    case 2: return "z" + k;
    case 3: return "w" + k;
  }
}

function role_ArithmeticExtensionGate(i,params) { 
  let numops = params[0];
  if (i>=8*numops) return "unused"; 
  let k  = div(i,8);
  let ir = ((i%2) == 0) ? 'r' : 'i';
  let j  = div(i,2);
  switch(j%4) { 
    case 0: return "x" + ir + k;
    case 1: return "y" + ir + k;
    case 2: return "z" + ir + k;
    case 3: return "w" + ir + k;
  }
}

function role_LookupGate(i,params) { 
  let numops = params[0];
  if (i>=2*numops) return "unused";
  let k = div(i,2);
  switch(i%2) { 
    case 0: return "inp" + k;
    case 1: return "out" + k;
  }
}

function role_LookupTableGate(i,params) {
  let numops = params[0];
  if (i>=3*numops) return "unused";
  let k = div(i,3);
  switch(i%3) { 
    case 0: return "inp"  + k;
    case 1: return "out"  + k;
    case 2: return "mult" + k;
  }
}

function role_NoopGate(i,params) { return "unused"; }

function role_ConstantGate(i,params) { 
  let nconsts = params[0]
  if (i>=nconsts) return "unused"
  return ("x" + i);
}

function role_PoseidonGate(i,params) {
  if (i< 12) { return "input"  + (i   ); }
  if (i< 24) { return "output" + (i-12); }
  if (i==24) { return "swap_flag";       }
  if (i< 29) { return "delta" + (i-25);  }
  if (i< 65) { return "round=" + (div((i-29),12)+1) + " sbox" + ((i-29)%12) + "_in"; }
  if (i< 87) { return "round=" + (i-65+4) + " sbox_in"; } 
  return "round=" + (div((i-87),12)+26) + " sbox" + ((i-87)%12) + "_in"; 
}

function role_BaseSumGate(i,params) {
  let radix = params[0];
  if (i>=radix+1) return "unused";
  if (i==0) return "sum";
  return ("limb" + (i-1));
}

function role_MultiplicationExtensionGate(i,params) { 
  let numops = params[0];
  if (i>=6*numops) return "unused"; 
  let k  = div(i,6);
  let ir = ((i%2) == 0) ? 'r' : 'i';
  let j  = div(i,2);
  switch(j%3) { 
    case 0: return "x" + ir + k;
    case 1: return "y" + ir + k;
    case 2: return "z" + ir + k;
  }
}

function role_PublicInputGate(i,params) {
  if (i>=4) return "unused";
  return ("hash" + i); 
}

function role_ExponentiationGate(i,params) {
  let exp_bits = params[0];
  if (i==0)            return "base";
  if (i<=exp_bits)     return "expo" + (i-1);
  if (i==exp_bits+1)   return "output"
  if (i< 2*exp_bits+2) return "tmp" + (i-exp_bits-2);
  return "unused";
}

function role_CosetInterpolationGate      (i,params) { return "dunno"; }
function role_MultiplicationExtensionGate (i,params) { return "dunno"; }
function role_PoseidonMdsGate             (i,params) { return "dunno"; }
function role_RandomAccessGate            (i,params) { return "dunno"; }
function role_ReducingGate                (i,params) { return "dunno"; }
function role_ReducingExtensionGate       (i,params) { return "dunno"; }

const the_roles = 
  [ role_Unknown
  , role_ArithmeticGate
  , role_ArithmeticExtensionGate
  , role_BaseSumGate
  , role_ConstantGate
  , role_CosetInterpolationGate
  , role_ExponentiationGate
  , role_LookupGate
  , role_LookupTableGate
  , role_MultiplicationExtensionGate 
  , role_NoopGate
  , role_PoseidonGate
  , role_PoseidonMdsGate
  , role_PublicInputGate
  , role_RandomAccessGate
  , role_ReducingGate
  , role_ReducingExtensionGate
  ];

const the_gate_equations = 
  [ "???"                  // UnknownGate   
  , "w = c0*x*y + c1*z"    // ArithmeticGate
  , "w = c0*x*y + c1*z"    // ArithmeticExtensionGate 
  , "y = sum_i 2^i*b_i"    // BaseSumGate    
  , "x0=c0, x1=c1"         // ConstantGate                
  , "..."                  // CosetInterpolationGate      
  , "y = x^k"              // ExponentiationGate          
  , "(x,y) in T"           // LookupGate                  
  , "N/A"                  // LookupTableGate             
  , "z = c0*x*y"           // MultiplicationExtensionGate 
  , "true"                 // NoopGate                    
  , "..."                  // PoseidonGate                
  , "..."                  // PoseidonMdsGate             
  , "x[0..3] = hash(PI)"   // PublicInputGate             
  , "y = x[i]"             // RandomAccessGate            
  , "y = sum_ a^i*c_i"     // ReducingGate         
  , "y = sum_ a^i*c_i"     // ReducingExtensionGate
  ];

function findGateIndex(gate) {
  let k = the_supported_gates.indexOf(gate);
  return ((k<0) ? 0 : k);
} 

function add_table_row(table, idx, text, count, color) {
  let tr = document.createElement("tr");
  tr.style.background = (color)?color:"#fffff";

  let td1 = document.createElement("td"); td1.innerHTML = idx.toString();
  let td2 = document.createElement("td"); td2.innerHTML = text;
  let td3 = document.createElement("td"); td3.innerHTML = count.toString();

  tr.appendChild(td1);
  tr.appendChild(td2);
  tr.appendChild(td3);
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

var gates_base   = []; 
var gates_params = []; 
var gate_colors  = [];
var gate_indices = [];       // index into our tables from the selector index
var gate_counts  = [];  

function cell_hover(row,column) {
  let el_pos  = document.getElementById("cell-pos"   );
  let el_val  = document.getElementById("cell-value" );
  let el_gate = document.getElementById("cell-gate"  );
  let el_equ  = document.getElementById("cell-equ"   );
  let el_role = document.getElementById("cell-role"  );
  let sel    = selectors[row];
  let gate   = gate_indices[sel];
  let params = gates_params[sel];
  //console.log(gates[sel] + " : " + params);
  el_pos.innerHTML  = "row=" + row.toString() + " , col=" + column.toString();
  el_val.innerHTML  = matrix[column][row].toString();
  el_gate.innerHTML = gates[sel];
  el_equ.innerHTML  = the_gate_equations[gate];
  el_role.innerHTML = the_roles[gate](column,params);

  el_gate.style.background = gate_colors[sel];
}

// parseInt behaves like seriously WTF here...
function myParseInt(what) {
  if (typeof what == "string") {
    return parseInt(what);
  }
  else {
    return what;
  }
}

// from String to BigInt
function convertMatrix(matrix) {
  return matrix.map( (vector) => vector.map(BigInt) );
}

function initialize_from_witness(fname,json) {
  //console.log(json);

  let el_fname = document.getElementById("fname");
  el_fname.value = fname;

  // these are global variables!
  gates     = json.gates;
  matrix    = convertMatrix(json.matrix);
  selectors = json.selector_vector.map(myParseInt);
  ngates    = gates.length;
  nrows     = selectors.length;
  ncolumns  = matrix.length
  ncells    = ncolumns * nrows;

  let el_num_rows = document.getElementById("num-rows");
  let el_num_cols = document.getElementById("num-cols");
  el_num_rows.innerHTML = nrows.toString();
  el_num_cols.innerHTML = ncolumns.toString();

  for(let i=0; i<gates.length; i++) {
    let full  = gates[i];
    let parts = full.split(":");
    let base  = parts[0];
    parts.shift();              // remove the first element
    gates_base[i]   = base;
    gates_params[i] = parts.map(parseInt);
    k = findGateIndex(base);
    gate_indices[i] = k;
    gate_colors[i]  = the_gate_colors[k];
    gate_counts[i]  = 0;
  }

  for(let i=0; i<nrows; i++) {
    gate_counts[selectors[i]] ++;
  }

  let el_table = document.getElementById("gates");
  for(let i=0; i<ngates; i++) {
    add_table_row(el_table, i, gates[i], gate_counts[i], gate_colors[i]);
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

  let el_ratio = document.getElementById("nonempty-ratio");
  let ratio = Math.round( 100 * (ncells - empty_counter) / ncells);
  el_ratio.innerHTML = ratio.toString() + "%";
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

