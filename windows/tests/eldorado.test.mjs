import assert from 'node:assert/strict';
import {test,beforeEach,afterEach,mock} from 'node:test';
import {installFakeDom} from './fakedom.mjs';
import {State,DEFAULT_SETTINGS} from '../src/core/state.ts';
import {handleIntegrationUpdate,flushEldoradoAlert,scheduleEldoradoAlert} from '../src/island/integrations.ts';
import {buildEldorado,eldoradoCard,eldoradoMoney,ELDORADO} from '../src/views/eldorado.ts';
let calls,island;
beforeEach(()=>{
  installFakeDom();mock.timers.enable({apis:['setTimeout']});
  State.tasks=[];State.focusId=null;State.paused=false;State.pendingApproval=null;State.view='overview';State.mode='compact';State.os='windows';
  State.settings={...DEFAULT_SETTINGS,activeIntegrations:[ELDORADO]};State.integrations[ELDORADO]={data:{},loaded:false,configured:true,error:null};State.loadIntegrationTasks();
  State.eldoradoAlertTab='orders';State.eldoradoAlertSequence=0;
  calls=[];island={reveal:()=>calls.push('reveal'),alert:view=>{calls.push(view);State.view=view;State.mode='expanded';}};
  flushEldoradoAlert(island);
});
afterEach(()=>{State.settings.eldoradoPopups=false;flushEldoradoAlert(island);mock.timers.reset();});
const event=()=>({id:ELDORADO,data:{orders:[],notifications:[],newOrders:1,newNotifications:0},error:null,event:{success:true,label:'New Eldorado Order',detail:'Example item'}});
test('initial data populates the card silently, new activity opens the Eldorado view',()=>{
  handleIntegrationUpdate(island,{...event(),event:null});assert.deepEqual(calls,[]);
  handleIntegrationUpdate(island,event());assert.ok(calls.includes('eldorado'));assert.equal(State.focusId,ELDORADO);
});
test('an approval is never displaced; the queued popup opens after it ends',()=>{
  State.pendingApproval={requestId:'test',pillId:'agent_codex'};State.view='approval';State.mode='expanded';
  handleIntegrationUpdate(island,event());assert.ok(!calls.includes('eldorado'));assert.equal(State.view,'approval');
  State.pendingApproval=null;flushEldoradoAlert(island);assert.equal(State.view,'eldorado');
  const count=calls.length;flushEldoradoAlert(island);assert.equal(calls.length,count);
});
test('queued alerts wait until approval cleanup has finished navigating',()=>{
  State.pendingApproval={requestId:'test',pillId:'agent_codex'};State.view='approval';State.mode='expanded';
  handleIntegrationUpdate(island,event());
  State.pendingApproval=null;scheduleEldoradoAlert(island);
  State.view='overview';
  assert.ok(!calls.includes('eldorado'));
  mock.timers.tick(0);assert.equal(State.view,'eldorado');
});
test('disabled popups, paused monitoring and disabled connection do not open a popup',()=>{
  State.settings.eldoradoPopups=false;handleIntegrationUpdate(island,event());assert.ok(!calls.includes('eldorado'));
  calls=[];State.paused=true;handleIntegrationUpdate(island,event());assert.deepEqual(calls,[]);
  State.paused=false;State.settings.activeIntegrations=[];handleIntegrationUpdate(island,event());assert.deepEqual(calls,[]);
});
test('a composed chat is not interrupted by a service alert',()=>{
  State.view='prompt';State.mode='expanded';handleIntegrationUpdate(island,event());assert.ok(!calls.includes('eldorado'));
  State.view='overview';flushEldoradoAlert(island);assert.ok(calls.includes('eldorado'));
});
test('Eldorado views render remote text as text, retain errors and offer both lists',()=>{
  State.integrations[ELDORADO]={configured:true,loaded:true,error:null,data:{checkedAt:Date.now(),orders:[{id:'1',title:'<script>untrusted</script>',buyer:'Buyer',amount:12.5,currency:'USD',state:'Paid'}],notifications:[{id:'n',event:'OrderDisputed',title:'A dispute',unread:true}]}};
  let opened=false;const summary=eldoradoCard(()=>opened=true);assert.ok(summary.textContent.includes('<script>untrusted</script>'));summary.find('BUTTON')[0].fire('click');assert.ok(opened);
  const view=buildEldorado({collapse(){}});view.sync();assert.ok(view.el.textContent.includes('Buyer'));assert.equal(view.el.find('.eldorado-row').length,1);
  view.el.find('BUTTON').find(b=>b.textContent==='Notifications').fire('click');assert.ok(view.el.textContent.includes('Order Disputed'));assert.ok(view.el.textContent.includes('Unread'));
  assert.equal(eldoradoMoney({amount:1,currency:'invalid-currency'}),'1.00 invalid-currency');
});
test('a new account notification opens the Notifications tab without overriding a later manual selection',()=>{
  const view=buildEldorado({collapse(){}});
  const update=event();update.data.newOrders=0;update.data.newNotifications=1;update.data.notifications=[{id:'n',event:'OrderDisputed',title:'Test'}];
  handleIntegrationUpdate(island,update);view.sync();assert.ok(view.el.textContent.includes('Order Disputed'));
  view.el.find('BUTTON').find(b=>b.textContent==='Orders').fire('click');assert.ok(view.el.textContent.includes('No recent orders'));
  view.sync();assert.ok(view.el.textContent.includes('No recent orders'));
});
