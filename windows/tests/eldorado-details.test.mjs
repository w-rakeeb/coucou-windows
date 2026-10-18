import assert from 'node:assert/strict';
import {test} from 'node:test';
import {salesStats,salesDetails} from '../src/views/eldorado-details.ts';
import {installFakeDom} from './fakedom.mjs';
const order=(id,state='Completed',amount=10,currency='USD',extra={})=>({id,state,amount,currency,category:'Items',createdAt:'2026-10-18T10:00:00Z',...extra});
test('sales exclude cancellations, pending deliveries and refunds, keep currencies separate and deduplicate',()=>{
 const stats=salesStats([order('a'),order('a','Received',12.5),order('b','Completed',5,'EUR'),order('c','Canceled',100),order('d','Delivered',30),order('e','Completed',40,'USD',{refunded:true})]);
 assert.equal(stats.orders,5);assert.equal(stats.sales,2);assert.equal(stats.cancelled,1);assert.equal(stats.active,1);assert.equal(stats.refunded,1);
 assert.deepEqual(stats.totals,[{currency:'EUR',amount:5,average:5},{currency:'USD',amount:12.5,average:12.5}]);assert.deepEqual(stats.categories,[['Items',2]]);
});
test('periods use order dates; invalid and missing amounts never become revenue',()=>{
 const now=Date.parse('2026-10-18T12:00:00Z');const list=[order('a'),order('b','Completed',1,'USD',{createdAt:'2026-09-01T00:00:00Z'}),order('c','Completed',NaN),order('d','Completed',-10),order('e','Completed',2,'bad')];
 const stats=salesStats(list,7,now);assert.equal(stats.sales,4);assert.equal(stats.missingAmounts,3);assert.equal(stats.totals[0].amount,10);assert.equal(salesStats([]).totals.length,0);
});
test('category ranking counts sales and does not interpret remote markup',()=>{
 installFakeDom();const list=[order('a','Completed',2,'USD',{category:'<script>unsafe</script>'}),order('b','Completed',3,'USD',{category:'Accounts'})];
 const el=salesDetails(list,v=>String(v.amount),0,false,false,'Partial API response',()=>{});
 assert.ok(el.textContent.includes('<script>unsafe</script>'));assert.ok(el.textContent.includes('Partial history'));assert.ok(el.textContent.includes('Before fees'));assert.equal(el.find('PROGRESS').length,2);
});
import {Bridge} from '../src/core/bridge.ts';
import {buildEldorado,ELDORADO} from '../src/views/eldorado.ts';
import {State,DEFAULT_SETTINGS} from '../src/core/state.ts';
test('Details loads full history, merges live state, keeps period selection and refreshes safely',async()=>{
 installFakeDom();State.settings={...DEFAULT_SETTINGS};State.view='eldorado';State.eldoradoAlertSequence=0;
 State.integrations[ELDORADO]={configured:true,loaded:true,error:null,data:{orders:[order('a','Canceled')],checkedAt:Date.now()}};
 const saved=Bridge.eldoradoDetails;let calls=0;
 Bridge.eldoradoDetails=async()=>{calls++;return {orders:[order('a'),order('b')],complete:true,warning:''};};
 try{
 const view=buildEldorado({collapse(){}});view.sync();view.el.find('BUTTON').find(b=>b.textContent==='Details').fire('click');
 await new Promise(resolve=>setImmediate(resolve));
 assert.ok(view.el.textContent.includes('All available history'));assert.ok(view.el.textContent.includes('2 orders'));
 assert.equal(view.el.find('.eldorado-metrics')[0].find('STRONG')[0].textContent,'1');
 view.el.find('BUTTON').find(b=>b.textContent==='7 Days').fire('click');view.sync();
 assert.ok(view.el.textContent.includes('Top Categories'));assert.equal(calls,1);
 }finally{Bridge.eldoradoDetails=saved;}
});
test('Details keeps available live results and clearly labels a history failure',async()=>{
 installFakeDom();State.view='eldorado';State.eldoradoAlertSequence=0;State.integrations[ELDORADO]={configured:true,loaded:true,error:null,data:{orders:[order('a')]}};
 const saved=Bridge.eldoradoDetails;Bridge.eldoradoDetails=async()=>{throw Error('Unavailable');};
 try{const view=buildEldorado({collapse(){}});view.sync();view.el.find('BUTTON').find(b=>b.textContent==='Details').fire('click');await new Promise(resolve=>setImmediate(resolve));assert.ok(view.el.textContent.includes('Partial history'));assert.ok(view.el.textContent.includes('Could not load history'));assert.ok(view.el.textContent.includes('1 order'));}
 finally{Bridge.eldoradoDetails=saved;}
});
