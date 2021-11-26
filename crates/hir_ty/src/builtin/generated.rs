//! Generated by `generate_builtins`, do not edit by hand.

use super::*;
use hir_def::BuiltIn;
const BUILTIN_INFO: [BuiltinInfo; 116usize] = [
    ANALYSIS,
    ACOS,
    ACOSH,
    AC_STIM,
    ASIN,
    ASINH,
    ATAN,
    ATAN2,
    ATANH,
    COS,
    COSH,
    DDT,
    DDX,
    EXP,
    FLICKER_NOISE,
    FLOOR,
    FLOW,
    POTENTIAL,
    HYPOT,
    IDT,
    IDTMOD,
    LAPLACE_ND,
    LAPLACE_NP,
    LAPLACE_ZD,
    LAPLACE_ZP,
    LIMEXP,
    LN,
    LOG,
    MAX,
    MIN,
    NOISE_TABLE,
    NOISE_TABLE_LOG,
    POW,
    SIN,
    SINH,
    SQRT,
    TAN,
    TANH,
    LAST_CROSSING,
    SLEW,
    WHITE_NOISE,
    ABSDELAY,
    ZI_ND,
    ZI_NP,
    ZI_ZD,
    ZI_ZP,
    DISPLAY,
    STROBE,
    WRITE,
    MONITOR,
    DEBUG,
    FCLOSE,
    FOPEN,
    FDISPLAY,
    FWRITE,
    FSTROBE,
    FMONITOR,
    FGETS,
    FSCANF,
    SWRITE,
    SFORMAT,
    SSCANF,
    REWIND,
    FSEEK,
    FTELL,
    FFLUSH,
    FERROR,
    FEOF,
    FDEBUG,
    FINISH,
    STOP,
    FATAL,
    WARNING,
    ERROR,
    INFO,
    ABSTIME,
    DIST_CHI_SQUARE,
    DIST_EXPONENTIAL,
    DIST_POISSON,
    DIST_UNIFORM,
    DIST_ERLANG,
    DIST_NORMAL,
    DIST_T,
    RANDOM,
    ARANDOM,
    RDIST_CHI_SQUARE,
    RDIST_EXPONENTIAL,
    RDIST_POISSON,
    RDIST_UNIFORM,
    RDIST_ERLANG,
    RDIST_NORMAL,
    RDIST_T,
    CLOG2,
    LOG10,
    CEIL,
    TEMPERATURE,
    VT,
    SIMPARAM,
    SIMPROBE,
    DISCONTINUITY,
    LIMIT,
    BOUND_STEP,
    MFACTOR,
    XPOSITION,
    YPOSITION,
    ANGLE,
    HFLIP,
    VFLIP,
    PARAM_GIVEN,
    PORT_CONNECTED,
    ANALOG_NODE_ALIAS,
    ANALOG_PORT_ALIAS,
    TEST_PLUSARGS,
    VALUE_PLUSARGS,
    SIMPARAM_STR,
    ABS,
];
pub(crate) fn bultin_info(builtin: BuiltIn) -> BuiltinInfo {
    BUILTIN_INFO[builtin as u8 as usize]
}
