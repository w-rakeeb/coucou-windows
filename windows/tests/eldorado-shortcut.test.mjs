import assert from 'node:assert/strict';
import {test} from 'node:test';
import {installFakeDom} from './fakedom.mjs';
import {State,DEFAULT_SETTINGS} from '../src/core/state.ts';
import {Bridge} from '../src/core/bridge.ts';
import {buildOverview} from '../src/views/views.ts';
import {buildEldorado,ELDORADO} from '../src/views/eldorado.ts';
test('Eldorado has the same card Details shortcut as sessions and opens sales directly',async()=>{
 installFakeDom();window.addEventListener=()=>{};State.settings={...DEFAULT_SETTINGS,activeIntegrations:[ELDORADO]};State.os='windows';State.view='overview';State.mode='expanded';State.showingPlanDetail=false;State.eldoradoDetailsRequested=false;
 State.tasks=[];State.loadIntegrationTasks();State.focusId=ELDORADO;State.integrations[ELDORADO]={configured:true,loaded:true,error:null,data:{orders:[]}};
 const actions={setView:view=>{State.view=view;},blip(){},collapse(){},openSettingsWindow(){}};
 const saved=Bridge.eldoradoDetails;Bridge.eldoradoDetails=async()=>({orders:[],complete:true,warning:''});
 try {
 const overview=buildOverview(actions);overview.sync();const button=overview.el.find('.activity-open')[0];assert.equal(button.hidden,false);assert.equal(button.title,'View Eldorado sales details');
 button.fire('click');assert.equal(State.view,'eldorado');assert.equal(State.eldoradoDetailsRequested,true);
 const page=buildEldorado(actions);page.sync();await new Promise(resolve=>setImmediate(resolve));assert.equal(State.eldoradoDetailsRequested,false);assert.ok(page.el.textContent.includes('Gross Sales'));assert.ok(page.el.textContent.includes('Top Categories'));
 page.el.find('BUTTON').find(b=>b.textContent==='Back').fire('click');assert.equal(State.view,'overview');
 State.focusId='other';State.tasks=[{id:'other',name:'Other',state:'idle',steps:[],color:'#fff'}];overview.sync();assert.equal(button.hidden,true);
 State.tasks=[{id:'agent_codex_test',name:'Codex',source:'codex',state:'working',steps:['Working'],color:'#fff'}];State.focusId='agent_codex_test';overview.sync();assert.equal(button.hidden,false);button.fire('click');assert.equal(State.view,'activity');assert.equal(State.eldoradoDetailsRequested,false);
 }finally{Bridge.eldoradoDetails=saved;}
});
