<cfscript>
suiteBegin("Mixin writeback through an arguments-scope store (WireBox singleton-autowire)");

// Regression (WireBox port, stage-4 blocker). ColdBox/WireBox runtime-injects
// helper methods onto every autowire target via structAppend(target,mixins,true)
// inside MixerUtil.start(), invoked positionally as
//   variables.mixerUtil.start( arguments.target )
// On the singleton path Injector.autowire() first runs
//   arguments.targetID = arguments.mapping.getName();
// — a member-CALL result stored into the arguments scope. Two VM bugs combined:
//   #1 the deep variables-writeback for arguments.mapping.getName() injected a
//      spurious __variables key onto the *arguments scope* (treating a scope as
//      a CFC instance); then
//   #2 the arguments->locals param-sync (on the subsequent `arguments.x = ...`
//      store) copied that __variables over the frame's REAL component scope,
//      nulling variables.mixerUtil. start() then dispatched on null and silently
//      no-op'd, so the mixin was never injected and target.injectedMixin() threw
//      "has no function with name [injectedMixin]".
// Lucee (reference semantics) injects and resolves all of these fine.

inj = new MixinWBInjector();
m   = new MixinWBMapping( "ServiceA" );

// core: member-call RHS stored into arguments, then positional structAppend
a = new MixinWBTarget();
assert( "mixin injected after arguments-scope member-call store", inj.autowire( target = a, mapping = m ), true );

// control: targetID supplied so the member-call self-assign branch is skipped
b = new MixinWBTarget();
assert( "mixin injected on the targetID-supplied path", inj.autowire( target = b, mapping = m, targetID = "ServiceA" ), true );

// end-to-end: the injected method is actually callable
c = new MixinWBTarget();
assert( "injected mixin is invokable", inj.autowireAndCall( target = c, mapping = m ), "INJECTED:hi" );

// a brand-new arguments key (not a declared param) triggers the same path
d = new MixinWBTarget();
assert( "mixin injected after a new arguments key store", inj.autowireNewKey( target = d, mapping = m ), true );

// the helper reference held in `variables` must survive the arguments store
e = new MixinWBTarget();
assert( "variables.mixerUtil survives the arguments-scope store", inj.mixerSurvives( target = e, mapping = m ), true );

suiteEnd();
</cfscript>
