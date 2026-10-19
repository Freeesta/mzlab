/* Measure only (build.sh): the two calls the study needs from the IUPAC InChI library, exported to wasm.
   inchi_from_molfile returns "InChI\nAuxInfo\nInChIKey" in a static buffer. */
#include <stdlib.h>
#include <string.h>
#include "inchi_api.h"
static char out[65536];
__attribute__((export_name("alloc"))) void* walloc(int n){return malloc(n);}
__attribute__((export_name("inchi_from_molfile"))) const char* f(const char* mf){
  inchi_Output o; memset(&o,0,sizeof o);
  MakeINCHIFromMolfileText(mf, "", &o);
  out[0]=0;
  if (o.szInChI) { strncpy(out,o.szInChI,30000); strcat(out,"\n"); }
  if (o.szAuxInfo) strncat(out,o.szAuxInfo,30000);
  char key[32]={0},x1[65],x2[65];
  if (o.szInChI && GetINCHIKeyFromINCHI(o.szInChI,0,0,key,x1,x2)==0){strcat(out,"\n");strcat(out,key);}
  FreeINCHI(&o); return out;
}
